use std::collections::{HashMap, VecDeque};
use bson::doc;
use bson::oid::ObjectId;
use lib_core::model::spider::leadparams::Location;
use lib_core::model::spider::taskcheck::Check;
use lib_core::model::spider::worker::{UpdateWorkerStateReq, WorkerState};
use mongodb::Collection;
use mongodb::options::{FindOneAndUpdateOptions, ReturnDocument, UpdateOptions};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use ai::inference::model::completion_function::parse_context::req::chat_prospect_details;
use ai::inference::model::completion_function::parse_context::req_model::ProspectResp;
use lib_core::model::spider::company::{Company, CompanyBuilder};
use lib_core::model::spider::{insert, Insertable, MDB};
use lib_core::model::spider::prospect::Prospect;
use zion::get_context;
use crate::channel::process::google_places::search;
use crate::channel::task::errors::Error;
use crate::channel::task::stream_log::{send_log, StreamMsg};
use crate::config::config;


#[derive(Debug, Clone)]
pub struct TaskState {
    db: MDB,
    reqw_c: reqwest::Client,
    // -- identifiers
    id: ObjectId,
    uid: ObjectId,
    campaign_id: ObjectId,
    /// --- queued so that items in progress can be removed
    q_industries: VecDeque<String>,
    current_industry: Option<String>,
    finished_industries: Vec<String>,
    q_locations: VecDeque<Location>,
    locations: Vec<Location>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NewTask {
    id: ObjectId,
    uid: ObjectId,
    campaign_id: ObjectId,
    industries: Vec<String>,
    locations: Vec<Location>,
}

#[derive(Debug)]
pub enum CycleCmd {
    // --- successful cycle
    OK,
    // -- all combos crawled
    FINISHED,
    CONTINUE,
}

impl TaskState {
    pub fn new(nts: NewTask, db: MDB, reqw_c: reqwest::Client) -> Self {
        TaskState {
            db,
            reqw_c,
            id: nts.id,
            uid: nts.uid,
            campaign_id: nts.campaign_id,
            q_industries: nts.industries.into(),
            current_industry: None,
            finished_industries: vec![],
            q_locations: nts.locations.into(),
            locations: vec![],
        }
    }

    pub async fn cycle(&mut self, stream_tx: &Sender<bytes::Bytes>) -> Result<CycleCmd, Error> {
        let Ok(res_company) = self.find_uncrawled().await else {
            return Ok(CycleCmd::CONTINUE);
        };


        let company = match res_company {
            Some(c) => {
                send_log(stream_tx, &c.log_msg()).await;
                c
            },
            // -- if no more uncrawled companies, search next industry + location combo
            None => {
                let res = self.search_next(&stream_tx).await?;
                return Ok(res);
            },
        };

        let res = match get_context(&company.website).await {
            Ok(v) => v,
            Err(e) => {
                println!("error getting prospects -- {e}");
                return Err(Error::FailedCrawl);
            }
        };

        if res.len() > 0 {
            let msg = format!("synthesizing {} emails", res.len());
            send_log(stream_tx, &msg).await;
        }

        let prospects = extract_prospects(&self.reqw_c, res).await?;

        if prospects.len() > 0 {
            let msg = format!("found {} prospects", prospects.len());
            send_log(stream_tx, &msg).await;
        }

        if !prospects.is_empty() {
            let filter = doc!{ "_id" : company.id() };
            let update = doc!{ "$set" : { "prospects": prospects }};
            let options = UpdateOptions::builder().upsert(true).build();
            let Ok(_result) = &self.db
                .collection::<Collection<Company>>(Company::COL)
                .update_one(filter, update, Some(options))
                .await else {
                    return Err(Error::FailedToUdateCompany);
                };
        }

        Ok(CycleCmd::OK)
    }
    async fn search_next(&mut self, stream_tx: &Sender<bytes::Bytes>) -> Result<CycleCmd, Error> {
        let location = match self.q_locations.pop_front() {
            Some(l) => {
                self.locations.push(l.clone());
                l
            },
            None => {
                // --- update current industry or finish operation
                let Some(ci) = self.q_industries.pop_front() else {
                    return Ok(CycleCmd::FINISHED);
                };
                self.current_industry = Some(ci);
                // --- reset locations by putting them back into vecdeque;
                let vec_locations = self.locations.clone();
                self.q_locations = vec_locations.into();
                // -- return new collection
                let Some(loc) = self.q_locations.pop_front() else {
                    println!("ERROR --- unable to reset q_locations, no locations in locations vec");
                    return Ok(CycleCmd::FINISHED);
                };
                loc
            },
        };

        let industry = match &self.current_industry {
            Some(ci) => ci.to_owned(),
            None => {
                let Some(ci) = self.q_industries.pop_front() else {
                    return Ok(CycleCmd::FINISHED);
                };
                ci
            }
        };

        let Ok(companies) =
            search(&industry, &location.city, &location.state).await else {
            println!("failed to get companies");
            return Ok(CycleCmd::CONTINUE);
        };

        let msg = format!("finding {} organizations in {}, {}", &industry, &location.city, &location.state);
        send_log(stream_tx, &msg).await;

        for c in companies {
            // -- build new company
            let Some(company) = CompanyBuilder::new()
                .set_campaign_id(self.campaign_id)
                .set_uid(self.uid)
                .set_name(c.name.unwrap_or_default())
                .set_website(c.website.unwrap_or_default())
                .set_phone(c.formatted_phone_number)
                .set_industry(Some(industry.clone()))
                .set_address(c.formatted_address)
                .set_state(Some(location.state.clone()))
                .set_city(Some(location.city.clone()))
                .set_zip(None)
                .build() else {
                    return Err(Error::FailedToBuildCompany);
            };

            // -- inserts un-crawled business into mongo
            if insert(&self.db, &company).await.is_err() {
                return Err(Error::FailedToInsertCompany);
            };
        }

        Ok(CycleCmd::OK)
    }
    async fn find_uncrawled(&self) -> Result<Option<Company>, Error> {
        let col: Collection<Company> = self.db.collection(Company::COL);
        let filter= doc!{
                "campaign_id" : self.campaign_id,
                "crawled": false
        };
        let update = doc!{
            "$set": { "crawled" : true },
        };
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        match col.find_one_and_update( filter, update, options ).await {
            Ok(r) => Ok(r),
            Err(e) => {
                println!("error -- unable to get uncrawled; cause ; {e}");
                Err(Error::FailedToGetCompany)
            },
        }
    }

    pub fn add_industry(&mut self, i: String){
        self.q_industries.push_back(i);
    }

    pub fn add_location(&mut self, i: Location){
        self.q_locations.push_back(i);
    }

    pub fn loc(&self) -> Vec<Location> {
        self.locations.clone()
    }

    pub fn ind(&self) -> VecDeque<String> {
        self.q_industries.clone()
    }
    
    pub async fn finish(&self) -> Result<(), Error> {
        let config = config();
        let url = format!("{}/update-worker-state", config.WEB_SERVER_URL);

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let req = UpdateWorkerStateReq {
            cid: self.campaign_id,
            worker_id: self.id,
            state: WorkerState::Finished,
        };

        if self.reqw_c
            .put(url)
            .headers(headers)
            .json(&req)
            .send()
            .await
            .is_err() {
                println!("failed to update worker state");
                return Err(Error::FailedUpdateWorkerState);
        }

        Ok(())
    }
}

pub trait Checkable {
    fn check(&self) -> Option<Check>;
}

impl Checkable for Option<TaskState> {
    fn check(&self) -> Option<Check> {
        let cloned = self.as_ref().clone();
        if let Some(ts) = cloned {
            return Some(
                Check {
                    current_industry: ts.current_industry.to_owned(),
                    todo_industries: ts.q_industries.to_owned().into(),
                    finished_industries: ts.finished_industries.to_owned(),
                    todo_locations: ts.locations.to_owned().into(),
                    finised_locations: ts.to_owned().locations.clone(),
                }
            )
        }
        None
    }
}

async fn extract_prospects(reqw_c: &reqwest::Client, context: HashMap<String, String>) -> Result<Vec<Prospect>, Error> {
    let mut prospects: Vec<Prospect> = Vec::new();

    // -- associate people to contact info
    for (email, html) in context {
        match chat_prospect_details(reqw_c, &email, &html).await {
            Ok(res) => {
                let response = match res.json::<ai::inference::model::completion_function::response::Response>().await {
                    Ok(r)=> r,
                    Err(e) => {
                        println!("ERROR!! --- response -- {e}");
                        return Err(Error::FailedProspectExtraction)
                    }
                };

                let choice= &response.choices[0];

                let args = match &choice.message.function_call {
                    Some(fc) => &fc.arguments,
                    None => {
                        let Some(content) = &choice.message.content else {
                            println!("error -- no function call or content");
                            continue;
                        };
                        content
                    }
                };

                let prospect_resp = match serde_json::from_str::<ProspectResp>(&args) {
                    Ok(r) => r,
                    Err(e) => {
                        println!("unable to convert prospect args to ProspectResp === {e}");
                        continue;
                    }
                };

                let p = Prospect::new()
                    .set_email(&email)
                    .set_context(&html)
                    .set_first_name(prospect_resp.firstname)
                    .set_last_name(prospect_resp.lastname)
                    .set_job_title(prospect_resp.jobtitle)
                    .set_score(prospect_resp.employee_score)
                    .build();

                prospects.push(p)
            },

            Err(e) => {
                println!("failed prospect extraction ; cause ; {}", e);
                return Err(Error::FailedProspectExtraction)
            },
        }
    }
    Ok(prospects)
}
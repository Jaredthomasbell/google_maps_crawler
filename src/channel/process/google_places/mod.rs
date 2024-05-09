use std::collections::VecDeque;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::config::config;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GoogleDetailsResponse {
    pub html_attributions: Vec<String>,
    pub result: GoogleDetailsResult,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GoogleDetailsResult {
    pub formatted_address: Option<String>,
    pub formatted_phone_number: Option<String>,
    pub name: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GoogleTextSearchResponseInit {
    pub html_attributions: Vec<String>,
    pub next_page_token: Option<String>,
    pub results: Vec<GoogleTextSearchResults>,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GoogleTextSearchResults {
    pub name: String,
    pub place_id: String,
}
pub async fn search(industry: &str, city: &str, state: &str)
                                    -> Result<Vec<GoogleDetailsResult>, reqwest::Error> {
    let config = config();

    let query_text = format!("{}%20in%20{},%20{}", industry, city, state);

    println!("searching -- {}", query_text);

    let client = reqwest::Client::new();
    let url = format!(
        "https://maps.googleapis.com/maps/api/place/textsearch/json?query={}&key={}",
        query_text, config.GOOGLE_PLACES_API
    );
    let mut place_data: Vec<GoogleTextSearchResults> = Vec::new();
    let mut next_page_token: VecDeque<String> = VecDeque::new();

    let mut _init_response_ = client.get(&url)
        .send()
        .await
        .unwrap()
        .json::<GoogleTextSearchResponseInit>()
        .await?;

    place_data.append(&mut _init_response_.results);

    if let Some(next_pg_token) = _init_response_.next_page_token {
        next_page_token.push_back(next_pg_token);
    }

    std::thread::sleep(Duration::from_secs(2));

    while let Some(pagetoken) = next_page_token.pop_front() {
        std::thread::sleep(Duration::from_secs(2));
        let url = format!("{}&pagetoken={}",url, &pagetoken);

        let mut _next_page_response_ = client.get(url)
            .send()
            .await
            .unwrap()
            .json::<GoogleTextSearchResponseInit>()
            .await?;

        place_data.append(&mut _init_response_.results);

        if let Some(nxt_pg) = _next_page_response_.next_page_token {
            next_page_token.push_back(nxt_pg)
        }
    }

    let mut response_vec: Vec<GoogleDetailsResult> = Vec::new();

    for place_id in place_data {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let url = format!(
            "https://maps.googleapis.com/maps/api/place/details/json?place_id={}&fields=website,formatted_address,name,formatted_phone_number&key={}",
            place_id.place_id, config.GOOGLE_PLACES_API);

        let mut _place_search_ = client.get(url)
            .send()
            .await
            .unwrap()
            .json::<GoogleDetailsResponse>()
            .await?;

        if _place_search_.result.website.is_some() {
            response_vec.push(_place_search_.result)
        }
    }

    Ok(response_vec)
}




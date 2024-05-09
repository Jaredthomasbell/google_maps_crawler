use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use lib_core::model::spider::leadparams::Location;
use lib_core::model::spider::taskcheck::CheckStateSender;
use lib_core::model::spider::value::Value;
use tokio::sync::{Mutex, Notify};
use tokio::sync::mpsc::Receiver;
use chrono::prelude::*;
use lib_core::model::spider::{mdb, MDB};
use crate::channel::cmd::Cmd;
use crate::channel::task::locked_taskstate::LockedTaskState;
use crate::channel::task::stream_log::StreamMsg;
use crate::channel::task::taskstate::{CycleCmd, TaskState};
use super::task::taskstate::Checkable;

pub mod google_places;

pub struct LeadGenRecord {
    uid: String,
    cid: String,
    created: DateTime<Utc>,
    result_count: Option<i32>,
    industries: Vec<String>,
    locations: Vec<Location>,
}

pub async fn process(rx: Receiver<Cmd>, tx: Arc<Mutex<CheckStateSender>>, stream_tx: Sender<bytes::Bytes> ) {
    // --- includes Notify so lead gen waits until start task is sent
    let lck_state = LockedTaskState::new();
    let lck_state2 = lck_state.arc_clone();
    //  --- make new clients
    let db = mdb().await;
    let reqw_c = reqwest::Client::new();

    //  --- handles incoming command and updates the task task
    tokio::spawn(async move {
        cmd_listener(tx, rx, lck_state, db, reqw_c).await;
    });
    // --- waits until notified to start generating leads for task task
    complete_cmd(lck_state2, &stream_tx).await;
}

async fn cmd_listener(
    // -- send response to server endpoint
    lck_sender: Arc<Mutex<CheckStateSender>>, 
    // -- recv command from server endpoint
    mut rx: Receiver<Cmd>, 
    lck_state: LockedTaskState, 
    db: MDB, 
    reqw_c: reqwest::Client
) {
    while let Some(cmd) = rx.recv().await {
        let (task, notify) = &*lck_state.state;
        let mut unlocked_task = task.lock().await;
        match cmd {
            Cmd::START(nt) => {
                let ts = TaskState::new(nt, db.clone(), reqw_c.clone());
                *unlocked_task = Some(ts);
            },
            Cmd::STOP => *unlocked_task = None,
            Cmd::CHECK => {
                let task = &*unlocked_task;
                let check = task.check();
                let sender = lck_sender.lock().await;
                let _send = sender.tx.send( check ).await;
            },
            Cmd::ADD(v) => {
                match v {
                    Value::Location(l) => {
                        if let Some(ts) = &mut *unlocked_task {
                            ts.add_location(l);
                            println!("{:#?} --- ADDED LOCATION?", ts);
                        }
                    },
                    Value::Industry(s) => {
                        if let Some(ts) = &mut *unlocked_task {
                            ts.add_industry(s);
                            println!("{:#?} --- ADDED INDUSTRY", ts);
                        }
                    },
                }
            }
            _ => panic!("unimplemented cmd type"),
        }
        notify.notify_waiters();
    }
}

async fn complete_cmd(lck_state2: LockedTaskState, stream_tx: &Sender<bytes::Bytes>) {
    loop {
        
        let (task, notify) = &*lck_state2.state;

        await_notify(task, notify).await;

        let Some(ts) = &mut *task.lock().await else {
            continue;
        };

        let c_cmd = match ts.cycle(stream_tx).await {
            Ok(c) => c,
            Err(e) => {
                println!("ERROR -- {:#?}", e);
                continue;
            }
        };

        match c_cmd {
            CycleCmd::CONTINUE => continue,
            CycleCmd::OK => continue,
            CycleCmd::FINISHED => {
                let mut t = task.lock().await;
                if let Some(ts) = &mut *t {
                    let _ = ts.finish().await
                        .expect("CRITICAL! -- finish task state request failed!!");
                } else {
                    panic!("NO STATAE ON FINISH??");
                }
                *t = None;
            }
        }
    }
}


async fn await_notify(task: &Mutex<Option<TaskState>>, notify: &Notify){
    let fut = notify.notified();
    tokio::pin!(fut);

    loop {
        fut.as_mut().enable();
        if let Some(_v) = &*task.lock().await{
            break;
        }
        fut.as_mut().await;
    }
}



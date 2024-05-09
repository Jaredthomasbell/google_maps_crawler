use lib_core::model::spider::value::Value;
use tokio::sync::mpsc::Sender;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use crate::channel::task::taskstate::NewTask;

pub struct CmdSender {
    pub tx: Sender<Cmd>,
}

impl CmdSender {
    pub(crate) fn arc(s_cmd: Sender<Cmd>) -> Arc<Mutex<CmdSender>> {
        Arc::new(Mutex::new(
            CmdSender {
                tx: s_cmd
            }
        ))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Cmd {
    START(NewTask),
    REMOVE(Value),
    ADD(Value),
    STOP,
    PAUSE,
    CHECK,
}
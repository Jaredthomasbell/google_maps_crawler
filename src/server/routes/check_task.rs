use std::sync::Arc;
use lib_core::model::spider::taskcheck::CheckStateReciever;
use tokio::sync::Mutex;
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bson::oid::ObjectId;
use serde::Deserialize;
use crate::channel::cmd::{Cmd, CmdSender};

#[derive(Debug, Deserialize)]
pub struct StartReq {
    uid: ObjectId,
    campaign_id: ObjectId,
    prompt: String,
}

pub async fn check_task_handler(
    Extension(lck_sender): Extension<Arc<Mutex<CmdSender>>>,
    Extension(lck_reciever): Extension<Arc<Mutex<CheckStateReciever>>>,
) -> Response {
    let cmd = Cmd::CHECK;
    let sender = lck_sender.lock().await;
    let _send = sender.tx.send( cmd ).await;

    let mut rcv = lck_reciever.lock().await;

    while let Some(cmd) = rcv.rx.recv().await {
        return Json(cmd).into_response();
    }

    StatusCode::BAD_REQUEST.into_response()
}
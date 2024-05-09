use std::sync::Arc;
use lib_core::model::spider::value::Value;
use tokio::sync::Mutex;
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use crate::channel::cmd::{Cmd, CmdSender};


pub async fn add_task_param_handler(
    Extension(lck_sender): Extension<Arc<Mutex<CmdSender>>>,
    Json(v): Json<Value>,
) -> Response {
    let cmd = Cmd::ADD(v);
    let sender = lck_sender.lock().await;
    let Ok(_send) = sender.tx.send( cmd ).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    StatusCode::OK.into_response()
}
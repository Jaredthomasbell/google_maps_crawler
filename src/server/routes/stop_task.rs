use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use crate::channel::cmd::{Cmd, CmdSender};

pub async fn stop_task_handler(
    Extension(lck_sender): Extension<Arc<Mutex<CmdSender>>>,
) -> Response {
    let cmd = Cmd::STOP;
    let task = lck_sender.lock().await;
    let _send = task.tx.send( cmd ).await;
    StatusCode::OK.into_response()
}
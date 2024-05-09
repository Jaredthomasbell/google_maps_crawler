use std::sync::Arc;
use axum::body::StreamBody;
use axum::Extension;
use axum::response::{IntoResponse, Response};
use crate::channel::task::stream_log::{stream_log, StreamReceiver};
use tokio::sync::Mutex;

pub async fn stream_log_handler(
    Extension(lck_receiver): Extension<Arc<Mutex<StreamReceiver>>>,
) -> Response {

    let mut stream = stream_log(lck_receiver);

    Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Transfer-Encoding", "chunked")
        .header("Connection", "keep-alive")
        .body(StreamBody::new(stream))
        .unwrap()
        .into_response()

}
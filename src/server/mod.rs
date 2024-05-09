mod routes;

use std::net::SocketAddr;
use axum::Router;
use axum::routing::{post, get, put};
use lib_core::model::spider::taskcheck::CheckStateReciever;
use crate::channel::cmd::CmdSender;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tower_http::add_extension::AddExtensionLayer;
use tower::ServiceBuilder;
use routes::start_task::start_task_handler;
use routes::stop_task::stop_task_handler;
use crate::channel::task::stream_log::{StreamMsg, StreamReceiver};
use crate::server::routes::stream_log::stream_log_handler;

use self::routes::add_param::add_task_param_handler;
use self::routes::check_task::check_task_handler;


pub async fn server(
    sender: Arc<Mutex<CmdSender>>,
    receiver: Arc<Mutex<CheckStateReciever>>,
    stream_receiver: Arc<Mutex<StreamReceiver>>)
{
    let app =
        task(sender, receiver)
        .merge(streamer(stream_receiver));

    let addr = "[::]:8001".parse::<SocketAddr>().unwrap();
    println!("{:<4} - {addr}\n", "LISTENING");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn task( sender: Arc<Mutex<CmdSender>>, receiver: Arc<Mutex<CheckStateReciever>>) -> Router {
    Router::new()
        .route("/start-task", post(start_task_handler))
        .route("/stop-task", post(stop_task_handler))
        .route("/check-task", get(check_task_handler))
        .route("/add-param", put(add_task_param_handler))
        .layer(ServiceBuilder::new()
            .layer(AddExtensionLayer::new(sender)))
        .layer(ServiceBuilder::new())
            .layer(AddExtensionLayer::new(receiver))
}

fn streamer( stream_sender: Arc<Mutex<StreamReceiver>> ) -> Router {
    Router::new()
        .route("/stream-log", post(stream_log_handler))
        .layer(ServiceBuilder::new()
            .layer(AddExtensionLayer::new(stream_sender)))
}

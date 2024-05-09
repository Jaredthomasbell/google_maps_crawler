pub mod config;
mod channel;
mod server;

use std::sync::Arc;

use lib_core::model::spider::taskcheck::{CheckStateSender, CheckStateReciever};
use tokio::sync::{mpsc, Mutex};
use crate::channel::cmd::CmdSender;
use crate::channel::process::process;
use crate::channel::task::stream_log::{StreamMsg, StreamReceiver};
use crate::server::server;

#[tokio::main]
async fn main() {
    let (server_tx, process_rx) = mpsc::channel(1);
    let (process_tx, server_rx) = mpsc::channel(1);
    let (stream_tx, stream_rx) = mpsc::channel::<bytes::Bytes>(1);

    let server_sender: Arc<Mutex<CmdSender>> = CmdSender::arc(server_tx);
    let proccess_sender: Arc<Mutex<CheckStateSender>> = CheckStateSender::arc(process_tx);

    let server_receiver: Arc<Mutex<CheckStateReciever>> = CheckStateReciever::arc(server_rx);

    let stream_receiver: Arc<Mutex<StreamReceiver>> = StreamReceiver::arc(stream_rx);

    tokio::spawn(async move {
        process(process_rx, proccess_sender, stream_tx).await
    });

    server(server_sender, server_receiver, stream_receiver).await;
}
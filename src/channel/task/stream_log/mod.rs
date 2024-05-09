use std::sync::Arc;
use bytes::Bytes;
use futures::{Stream, stream, StreamExt, TryStream};
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::sync::Mutex;

pub struct StreamReceiver {
    pub rx: Receiver<bytes::Bytes>,
}

impl StreamReceiver {
    pub fn arc(r: Receiver<bytes::Bytes>) -> Arc<Mutex<StreamReceiver>> {
        Arc::new(Mutex::new(
            StreamReceiver {
                rx: r
            }
        ))
    }

    pub fn stream(mut self) {
    }
}

pub struct StreamMsg {
   msg: bytes::Bytes,
}

impl Into<bytes::Bytes> for StreamMsg {
    fn into(self) -> Bytes {
        self.msg
    }
}

pub async fn send_log(tx: &Sender<bytes::Bytes>, msg: &str) {
    println!("sending stream log --- {msg}");
   let _ = tx.send(  bytes::Bytes::from(msg.to_owned()) ).await;
}


pub fn stream_log(lck_recv: Arc<Mutex<StreamReceiver>> ) -> impl Stream<Item = std::io::Result<Bytes>> + Sized + Unpin {

    let stream = stream::unfold(lck_recv, |r| async move {
        let Some(msg) = log_recv(&r).await else {
            return None;
        };
        println!("msg bytes --- {:#?}", msg);
        Some((Ok(Bytes::from(msg)), r))
    });

   Box::pin(stream)
}

pub async fn log_recv(lck_recv: &Arc<Mutex<StreamReceiver>> ) -> Option<String> {
    let mut receiver= lck_recv.lock().await;
    if let Some(stream_msg) = receiver.rx.recv().await {
        let msg_str = String::from_utf8(stream_msg.to_vec()).unwrap(); // Convert Bytes to String
        let sse_msg = format!("data: {}\n\n", msg_str); // Convert String to SSE message
        return Some(sse_msg)
    }
    println!("CRITICAL! LOG RECEIVER IS RETURNING NONE");
    None
}
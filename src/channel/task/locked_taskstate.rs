use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use crate::channel::task::taskstate::TaskState;

pub struct LockedTaskState {
    pub state: Arc<(Mutex<Option<TaskState>>, Notify)>,
}

impl LockedTaskState {
    pub fn new() -> Self {
        Self {
            state: Arc::new((Mutex::new(None), Notify::new()))
        }
    }
    pub fn arc_clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state)
        }
    }
}

pub mod states;

use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
    time::Instant,
};

use rocket::tokio::sync::broadcast::{
    error::{RecvError, SendError},
    Receiver, Sender,
};
use states::{ClockEvent, CounterEvent, ToggleEvent};
use uuid::Uuid;

use crate::component::Component;

#[derive(Debug, Clone)]
pub enum Event {
    DataLog(serde_json::Value),
    Clock(ClockEvent),
    Counter(CounterEvent),
    Toggle(ToggleEvent),
}

#[derive(Debug, Clone)]
pub struct LogEvent {
    pub timestamp: Instant,
    pub log_id: Uuid,
    pub component: Component,
    pub event: Event,
}
impl LogEvent {
    pub fn new(component: Component, event: Event) -> Self {
        Self {
            timestamp: Instant::now(),
            log_id: Uuid::new_v4(),
            component,
            event,
        }
    }
}

#[derive(Debug)]
pub struct MessageChannel<T: Clone> {
    send: Sender<T>,
    recv: Receiver<T>,
}
impl<T: Clone> From<Sender<T>> for MessageChannel<T> {
    fn from(send: Sender<T>) -> Self {
        Self {
            send: send.clone(),
            recv: send.subscribe(),
        }
    }
}
impl<T: Clone> MessageChannel<T> {
    pub fn send(&self, value: T) -> Result<usize, SendError<T>> {
        self.send.send(value)
    }
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        self.recv.recv().await
    }
}

#[derive(Debug, Clone)]
pub struct Shareable<T> {
    pub data: Arc<Mutex<T>>,
}
impl<T> From<T> for Shareable<T> {
    fn from(value: T) -> Self {
        Self {
            data: Arc::new(Mutex::new(value)),
        }
    }
}

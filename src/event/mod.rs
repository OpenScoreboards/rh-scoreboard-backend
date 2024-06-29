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
use states::{ClockEvent, CounterEvent, LabelEvent, ToggleEvent};
use uuid::Uuid;

use crate::component::Component;

#[derive(Debug, Clone)]
pub enum Event {
    Clock(ClockEvent),
    Counter(CounterEvent),
    Toggle(ToggleEvent),
    Label(LabelEvent),
    Reset,
}

#[derive(Debug, Clone)]
pub struct LogEvent {
    pub timestamp: Instant,
    pub log_id: Uuid,
    pub component: Component,
    pub event: Event,
}
impl LogEvent {
    pub fn new_now(component: Component, event: Event) -> Self {
        Self {
            timestamp: Instant::now(),
            log_id: Uuid::new_v4(),
            component,
            event,
        }
    }
    pub fn new(
        component: Component,
        event: Event,
        ts: Option<usize>,
        uuid: Option<String>,
    ) -> Self {
        let timestamp = ts
            .and_then(|ts| {
                let ts_str = ts.to_string();
                let mut deserializer = serde_json::Deserializer::from_str(&ts_str);
                serde_millis::deserialize(&mut deserializer).ok()
            })
            .unwrap_or_else(Instant::now);

        let log_id = uuid
            .and_then(|uuid| Uuid::parse_str(&uuid).ok())
            .unwrap_or(Uuid::new_v4());
        Self {
            timestamp,
            log_id,
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

pub mod states;

use std::{fmt::Debug, time::Instant};

use rocket::tokio::sync::broadcast::Sender;
use serde_json::Value;
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

pub fn handle_data_log<T: Fn() -> Value>(
    log_event: &LogEvent,
    component: Component,
    send: &Sender<LogEvent>,
    get_data: T,
) -> bool {
    if matches!(log_event.event, Event::DataLog(serde_json::Value::Null)) {
        send.send(LogEvent::new(component, Event::DataLog(get_data())))
            .unwrap();
        return true;
    }
    false
}

pub trait EventListener: Send + Sync + std::fmt::Debug {
    fn notify(&mut self, event: &LogEvent);
}

pub trait EventLogger: Send + Sync + std::fmt::Debug {
    fn log_event(&mut self, component: Component, event: Event);
}

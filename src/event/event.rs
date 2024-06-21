use std::time::Instant;

use uuid::Uuid;

use crate::component::{scoreboard::Component, Event};

#[derive(Debug)]
pub struct LogEvent {
    pub timestamp: Instant,
    pub log_id: Uuid,
    pub component: Component,
    pub event: Event,
}

pub trait EventListener: Send + Sync + std::fmt::Debug {
    fn notify(&mut self, event: &LogEvent);
}

pub trait EventLogger {
    fn log_event(&mut self, component: Component, event: Event);
}

pub mod states;

use std::{fmt::Debug, time::Instant};

use states::{ClockEvent, CounterEvent, ToggleEvent};
use uuid::Uuid;

use crate::component::Component;

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Clock(ClockEvent),
    Counter(CounterEvent),
    ToggleEvent(ToggleEvent),
}

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

use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ClockState {
    Stopped,
    Running,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ClockEvent {
    Set(Duration),
    Start,
    Stop,
    Expired,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum CounterEvent {
    Set(Duration),
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ToggleState {
    Active,
    Inactive,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ToggleEvent {
    Activate,
    Deactivate,
}

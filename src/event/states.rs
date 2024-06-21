use std::time::Duration;

use rocket::request::FromParam;
use serde::Serialize;
use strum::{EnumString, ParseError};

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ClockState {
    Stopped,
    Running,
}

#[derive(Debug, Clone, Copy, Serialize, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum ClockEvent {
    Set(Duration),
    Start,
    Stop,
    Expired,
}
impl<'a> FromParam<'a> for ClockEvent {
    type Error = ParseError;
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.try_into()
    }
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

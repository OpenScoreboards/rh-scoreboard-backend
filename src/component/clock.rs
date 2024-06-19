pub enum ClockState {
    Stopped,
    Running,
}

pub enum ClockEvent {
    Set,
    Start,
    Stop,
    Expired,
}

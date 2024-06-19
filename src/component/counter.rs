use std::time::Duration;

pub enum CounterEvent {
    Set(Duration),
    Increment,
    Decrement,
}

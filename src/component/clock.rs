use std::time::{Duration, Instant};

use event::{states::ClockState, LogEvent, MessageChannel, Shareable};
use rocket::serde::Serialize;
use serde::Deserialize;
use serde_json::{json, value::Serializer};
use serde_millis::Milliseconds;

use crate::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClockComponent {
    #[serde(skip_serializing, default)]
    name: String,
    pub state: ClockState,
    #[serde(with = "serde_millis")]
    pub last_state_change: Instant,
    #[serde(with = "serde_millis")]
    pub last_time_remaining: Duration,
}
impl ClockComponent {
    fn new(name: String) -> Self {
        ClockComponent {
            name,
            state: ClockState::Stopped,
            last_state_change: Instant::now(),
            last_time_remaining: Duration::from_secs(0),
        }
    }
    fn process_event(&mut self, event: &LogEvent) {
        use ClockEvent as E;
        use ClockState as S;

        let Event::Clock(clock_event) = &event.event else {
            return;
        };
        match (&self.state, clock_event) {
            (_, E::Set(duration)) => {
                self.last_state_change = event.timestamp;
                self.last_time_remaining = *duration;
            }
            (S::Running, E::Stop) => {
                self.state = S::Stopped;
                let time_elapsed = Instant::now() - self.last_state_change;
                self.last_time_remaining = self.last_time_remaining.saturating_sub(time_elapsed);
                self.last_state_change = event.timestamp;
            }
            (S::Stopped, E::Start) => {
                self.state = S::Running;
                self.last_state_change = event.timestamp;
            }
            (_, E::Expired) => {
                self.state = S::Stopped;
                self.last_state_change = event.timestamp;
                self.last_time_remaining = Duration::from_secs(0);
            }
            _ => {
                eprintln!(
                    "Clock event {clock_event:?} in state {:?} has no action",
                    &self.state
                );
            }
        }
    }
}

fn to_json_value<T: Milliseconds>(value: &T) -> Value {
    serde_millis::serialize(value, Serializer).expect("failed to serialize to milliseconds")
}

#[derive(Debug)]
pub struct GameClock {
    clock: Shareable<ClockComponent>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
    typed_data_channel: MessageChannel<Option<(ClockState, Instant, Duration)>>,
}
impl GameClock {
    pub fn new(
        event_send: Sender<LogEvent>,
        data_log_send: Sender<Value>,
        typed_data_send: Sender<Option<(ClockState, Instant, Duration)>>,
    ) -> Self {
        Self {
            clock: ClockComponent::new("game_clock".into()).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
            typed_data_channel: typed_data_send.into(),
        }
    }
    pub async fn run(mut self) {
        let clock = self.clock.clone();
        tokio::spawn(async move {
            loop {
                let Ok(Value::Null) = self.data_channel.recv().await else {
                    continue;
                };
                let clock = clock.data.lock().unwrap();
                let _ = self.data_channel.send(json!({
                    &clock.name: {
                        "last_time_remaining": to_json_value(&clock.last_time_remaining),
                        "last_state_change": to_json_value(&clock.last_state_change),
                        "state": &clock.state,
                    }
                }));
            }
        });
        let clock = self.clock.clone();
        tokio::spawn(async move {
            loop {
                let Ok(None) = self.typed_data_channel.recv().await else {
                    continue;
                };
                let clock = clock.data.lock().unwrap();
                let _ = self.typed_data_channel.send(Some((
                    clock.state,
                    clock.last_state_change,
                    clock.last_time_remaining,
                )));
            }
        });
        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if log_event.component != Component::Global(GlobalComponent::GameClock) {
                    continue;
                }
                self.clock.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}

#[derive(Debug)]
pub struct GameDependentClock {
    component: Component,
    clock: Shareable<ClockComponent>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
}
impl GameDependentClock {
    pub fn new(
        component: Component,
        name: &str,
        event_send: Sender<LogEvent>,
        data_log_send: Sender<Value>,
    ) -> Self {
        Self {
            component,
            clock: ClockComponent::new(name.into()).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
        }
    }
    pub async fn run(mut self) {
        let clock = self.clock.clone();
        tokio::spawn(async move {
            loop {
                let Ok(Value::Null) = self.data_channel.recv().await else {
                    continue;
                };
                let clock = clock.data.lock().unwrap();
                let _ = self.data_channel.send(json!({
                    &clock.name: {
                        "last_time_remaining": to_json_value(&clock.last_time_remaining),
                        "last_state_change": to_json_value(&clock.last_state_change),
                        "state": &clock.state,
                    }
                }));
            }
        });
        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if !matches!(
                    log_event,
                    LogEvent {
                        component: Component::Global(GlobalComponent::GameClock),
                        event: Event::Clock(
                            ClockEvent::Start | ClockEvent::Stop | ClockEvent::Expired
                        ),
                        ..
                    }
                ) && log_event.component != self.component
                {
                    continue;
                }
                self.clock.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}

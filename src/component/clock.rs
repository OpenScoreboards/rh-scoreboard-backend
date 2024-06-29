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

        if let Event::Reset = &event.event {
            self.state = ClockState::Stopped;
            self.last_state_change = event.timestamp;
            self.last_time_remaining = Duration::from_secs(0);
            return;
        }
        let Event::Clock(clock_event) = &event.event else {
            return;
        };
        match (&self.state, clock_event) {
            (_, E::Set(duration)) => {
                self.last_state_change = event.timestamp;
                self.last_time_remaining = *duration;
            }
            (S::Running, E::Stop(None)) => {
                self.state = S::Stopped;
                let time_elapsed = event.timestamp - self.last_state_change;
                self.last_time_remaining = self.last_time_remaining.saturating_sub(time_elapsed);
                self.last_state_change = event.timestamp;
            }
            (S::Running, E::Stop(Some(val))) => {
                self.state = S::Stopped;
                self.last_time_remaining = *val;
                self.last_state_change = event.timestamp;
            }
            (S::Stopped, E::Start(None)) if self.last_time_remaining != Duration::from_secs(0) => {
                self.state = S::Running;
                self.last_state_change = event.timestamp;
            }
            (S::Stopped, E::Start(Some(val))) if *val != Duration::from_secs(0) => {
                self.state = S::Running;
                self.last_state_change = event.timestamp;
                self.last_time_remaining = *val;
            }
            (_, E::Increment(val)) => {
                let time_elapsed = if let ClockState::Running = self.state {
                    event.timestamp - self.last_state_change
                } else {
                    Duration::from_secs(0)
                };
                self.last_time_remaining = self
                    .last_time_remaining
                    .saturating_sub(time_elapsed)
                    .saturating_add(*val);
                self.last_state_change = event.timestamp;
            }
            (_, E::Decrement(val)) => {
                let time_elapsed = if let ClockState::Running = self.state {
                    event.timestamp - self.last_state_change
                } else {
                    Duration::from_secs(0)
                };
                self.last_time_remaining = self
                    .last_time_remaining
                    .saturating_sub(time_elapsed)
                    .saturating_sub(*val);
                self.last_state_change = event.timestamp;
            }
            (_, E::Expired) => {
                self.state = S::Stopped;
                self.last_state_change = event.timestamp;
                self.last_time_remaining = Duration::from_secs(0);
            }
            _ => {}
        }
    }
    fn get_time_remaining(&self) -> Duration {
        let time_elapsed = Instant::now() - self.last_state_change;
        if matches!(self.state, ClockState::Running) {
            self.last_time_remaining.saturating_sub(time_elapsed)
        } else {
            self.last_time_remaining
        }
    }
}

fn to_json_value<T: Milliseconds>(value: &T) -> Value {
    serde_millis::serialize(value, Serializer).expect("failed to serialize to milliseconds")
}

fn start_data_channel_manager(
    clock: Shareable<ClockComponent>,
    mut data_channel: MessageChannel<Value>,
) {
    tokio::spawn(async move {
        loop {
            let Ok(Value::Null) = data_channel.recv().await else {
                continue;
            };
            let clock = clock.data.lock().unwrap();
            let time_remaining = clock.get_time_remaining().as_secs();
            let _ = data_channel.send(json!({
                &clock.name: {
                    "last_time_remaining": to_json_value(&clock.last_time_remaining),
                    "last_state_change": to_json_value(&clock.last_state_change),
                    "state": &clock.state,
                    "time_remaining": format!("{:0>2}:{:0>2}", (time_remaining / 60) % 60, time_remaining % 60)
                }
            }));
        }
    });
}

fn start_typed_data_channel_manager(
    clock: Shareable<ClockComponent>,
    mut data_channel: MessageChannel<Option<(ClockState, Instant, Duration)>>,
) {
    tokio::spawn(async move {
        loop {
            let Ok(None) = data_channel.recv().await else {
                continue;
            };
            let clock = clock.data.lock().unwrap();
            let _ = data_channel.send(Some((
                clock.state,
                clock.last_state_change,
                clock.last_time_remaining,
            )));
        }
    });
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
        start_data_channel_manager(self.clock.clone(), self.data_channel);
        start_expiry_watcher(
            Component::Global(GlobalComponent::GameClock),
            true,
            self.event_channel.sender(),
            self.typed_data_channel.sender(),
        );
        start_typed_data_channel_manager(self.clock.clone(), self.typed_data_channel);

        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if !Component::Global(GlobalComponent::GameClock)
                    .is_event_component_relevant(&log_event.component)
                {
                    continue;
                }
                self.clock.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}

pub fn start_expiry_watcher(
    component: Component,
    activate_siren: bool,
    event_sender: Sender<LogEvent>,
    clock_data_sender: Sender<Option<(ClockState, Instant, Duration)>>,
) {
    tokio::spawn(async move {
        let mut recv = clock_data_sender.subscribe();
        loop {
            let _ = clock_data_sender.send(None);
            loop {
                let Ok(Some((state, last_state_change, last_time_remaining))) = recv.recv().await
                else {
                    continue;
                };
                let ClockState::Running = state else {
                    break;
                };
                let time_elapsed = Instant::now() - last_state_change;
                if time_elapsed < last_time_remaining {
                    break;
                }
                event_sender
                    .send(LogEvent::new_now(
                        component,
                        Event::Clock(ClockEvent::Expired),
                    ))
                    .unwrap();
                if activate_siren {
                    event_sender
                        .send(LogEvent::new_now(
                            Component::Global(GlobalComponent::Siren),
                            Event::Toggle(ToggleEvent::Activate),
                        ))
                        .unwrap();
                    sleep(Duration::from_secs(2)).await;
                    event_sender
                        .send(LogEvent::new_now(
                            Component::Global(GlobalComponent::Siren),
                            Event::Toggle(ToggleEvent::Deactivate),
                        ))
                        .unwrap();
                }
                break;
            }
            sleep(Duration::from_millis(200)).await;
        }
    });
}

#[derive(Debug)]
pub struct GameDependentClock {
    component: Component,
    clock: Shareable<ClockComponent>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
    typed_data_channel: MessageChannel<Option<(ClockState, Instant, Duration)>>,
}
impl GameDependentClock {
    pub fn new(
        event_send: Sender<LogEvent>,
        data_log_send: Sender<Value>,
        component: Component,
        name: &str,
        typed_data_send: Sender<Option<(ClockState, Instant, Duration)>>,
    ) -> Self {
        Self {
            component,
            clock: ClockComponent::new(name.into()).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
            typed_data_channel: typed_data_send.into(),
        }
    }
    pub async fn run(mut self) {
        start_data_channel_manager(self.clock.clone(), self.data_channel);
        start_expiry_watcher(
            Component::Global(GlobalComponent::ShotClock),
            false,
            self.event_channel.sender(),
            self.typed_data_channel.sender(),
        );
        start_typed_data_channel_manager(self.clock.clone(), self.typed_data_channel);

        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if !matches!(
                    log_event,
                    LogEvent {
                        component: Component::Global(GlobalComponent::GameClock),
                        event: Event::Clock(
                            ClockEvent::Start(None) | ClockEvent::Stop(None) | ClockEvent::Expired
                        ),
                        ..
                    }
                ) && !self
                    .component
                    .is_event_component_relevant(&log_event.component)
                {
                    continue;
                }
                self.clock.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}

#[derive(Debug)]
pub struct StoppageClock {
    component: Component,
    clock: Shareable<ClockComponent>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
    typed_data_channel: MessageChannel<Option<(ClockState, Instant, Duration)>>,
}
impl StoppageClock {
    pub fn new(
        event_send: Sender<LogEvent>,
        data_log_send: Sender<Value>,
        component: Component,
        name: &str,
        typed_data_send: Sender<Option<(ClockState, Instant, Duration)>>,
    ) -> Self {
        Self {
            component,
            clock: ClockComponent::new(name.into()).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
            typed_data_channel: typed_data_send.into(),
        }
    }
    pub async fn run(mut self) {
        start_data_channel_manager(self.clock.clone(), self.data_channel);
        start_expiry_watcher(
            self.component,
            true,
            self.event_channel.sender(),
            self.typed_data_channel.sender(),
        );
        start_typed_data_channel_manager(self.clock.clone(), self.typed_data_channel);

        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if !self
                    .component
                    .is_event_component_relevant(&log_event.component)
                {
                    continue;
                }
                if let Event::Clock(ClockEvent::Start(Some(_))) = log_event.event {
                    self.event_channel
                        .send(LogEvent {
                            component: Component::Global(GlobalComponent::GameClock),
                            event: Event::Clock(ClockEvent::Stop(None)),
                            ..log_event
                        })
                        .expect("game clock stop message failed to send");
                }
                self.clock.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}

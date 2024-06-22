use std::time::{Duration, Instant};

use broadcast::Receiver;
use event::{handle_data_log, states::ClockState, LogEvent};
use rocket::serde::Serialize;
use serde::Deserialize;

use crate::*;

use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClockComponent {
    #[serde(skip_serializing, default)]
    name: String,
    pub state: ClockState,
    #[serde(with = "serde_millis")]
    pub last_state_change: Instant,
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
                let time_elapsed = self.last_state_change - Instant::now();
                self.last_time_remaining -= time_elapsed;
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
    fn get_data(&self) -> serde_json::Value {
        let mut map = Map::default();
        map.insert(self.name.clone(), serde_json::to_value(self).unwrap());
        serde_json::Value::Object(map)
    }
}

#[derive(Debug)]
pub struct GameClock {
    send: Sender<LogEvent>,
    receive: Receiver<LogEvent>,
    game_clock: ClockComponent,
}
impl GameClock {
    pub fn new(send: Sender<LogEvent>, receive: Receiver<LogEvent>) -> Self {
        Self {
            send,
            receive,
            game_clock: ClockComponent::new("game_clock".into()),
        }
    }
    pub async fn run(mut self) {
        while let Ok(log_event) = self.receive.recv().await {
            if matches!(log_event.event, Event::DataLog(serde_json::Value::Null)) {
                self.send
                    .send(LogEvent {
                        timestamp: Instant::now(),
                        log_id: Uuid::new_v4(),
                        component: Component::Global(GlobalComponent::GameClock),
                        event: Event::DataLog(self.game_clock.get_data()),
                    })
                    .unwrap();
                continue;
            }
            if log_event.component != Component::Global(GlobalComponent::GameClock) {
                continue;
            }
            self.game_clock.process_event(&log_event);
        }
    }
}

#[derive(Debug)]
pub struct GameDependentClock {
    component: Component,
    send: Sender<LogEvent>,
    receive: Receiver<LogEvent>,
    clock: ClockComponent,
}
impl GameDependentClock {
    pub fn new(
        component: Component,
        name: &str,
        send: Sender<LogEvent>,
        receive: Receiver<LogEvent>,
    ) -> Self {
        Self {
            component,
            send,
            receive,
            clock: ClockComponent::new(name.into()),
        }
    }
    pub async fn run(mut self) {
        while let Ok(log_event) = self.receive.recv().await {
            if handle_data_log(&log_event, self.component, &self.send, || {
                self.clock.get_data()
            }) {
                continue;
            }
            if !matches!(
                log_event,
                LogEvent {
                    component: Component::Global(GlobalComponent::GameClock),
                    event: Event::Clock(ClockEvent::Start | ClockEvent::Stop | ClockEvent::Expired),
                    ..
                }
            ) && log_event.component != self.component
            {
                continue;
            }
            self.clock.process_event(&log_event);
        }
    }
}

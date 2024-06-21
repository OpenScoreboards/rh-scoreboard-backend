use std::time::{Duration, Instant};

use broadcast::Receiver;
use event::{states::ClockState, EventListener, LogEvent};
use rocket::serde::Serialize;

use crate::*;

use super::*;

#[derive(Debug, Serialize)]
struct ClockComponent {
    #[serde(skip_serializing)]
    name: String,
    state: ClockState,
    #[serde(with = "serde_millis")]
    last_state_change: Instant,
    last_time_remaining: Duration,
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
    send_receive: (Sender<LogEvent>, Receiver<LogEvent>),
    game_clock: ClockComponent,
}
impl GameClock {
    pub fn new(send_receive: (Sender<LogEvent>, Receiver<LogEvent>)) -> Self {
        Self {
            send_receive,
            game_clock: ClockComponent::new("game_clock".into()),
        }
    }
    pub async fn run(mut self) {
        while let Ok(log_event) = self.send_receive.1.recv().await {
            if matches!(log_event.event, Event::DataLog(serde_json::Value::Null)) {
                self.send_receive
                    .0
                    .send(LogEvent {
                        timestamp: Instant::now(),
                        log_id: Uuid::new_v4(),
                        component: Component::GameClock,
                        event: Event::DataLog(self.game_clock.get_data()),
                    })
                    .unwrap();
                continue;
            }
            if log_event.component != Component::GameClock {
                continue;
            }
            self.game_clock.process_event(&log_event);
        }
    }
}

#[derive(Debug)]
pub struct ShotClock {
    send: Sender<LogEvent>,
    receive: Receiver<LogEvent>,
    shot_clock: ClockComponent,
}
impl ShotClock {
    pub fn new(send_receive: (Sender<LogEvent>, Receiver<LogEvent>)) -> Self {
        Self {
            send: send_receive.0,
            receive: send_receive.1,
            shot_clock: ClockComponent::new("shot_clock".into()),
        }
    }
    pub async fn run(mut self) {
        while let Ok(log_event) = self.receive.recv().await {
            if matches!(log_event.event, Event::DataLog(serde_json::Value::Null)) {
                self.send
                    .send(LogEvent::new(
                        Component::ShotClock,
                        Event::DataLog(self.shot_clock.get_data()),
                    ))
                    .unwrap();
                continue;
            }
            if log_event.component != Component::ShotClock {
                continue;
            }
            self.shot_clock.process_event(&log_event);
        }
    }
}

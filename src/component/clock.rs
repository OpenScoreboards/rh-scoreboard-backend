use std::time::{Duration, Instant};

use rocket::serde::Serialize;

use crate::{
    event::{states::*, Event, EventListener, LogEvent},
    ScoreboardComponent,
};

use super::*;

#[derive(Debug, Serialize)]
struct ClockComponent {
    state: ClockState,
    #[serde(with = "serde_millis")]
    last_state_change: Instant,
    last_time_remaining: Duration,
}
impl ClockComponent {
    fn new() -> Self {
        ClockComponent {
            state: ClockState::Stopped,
            last_state_change: Instant::now(),
            last_time_remaining: Duration::from_secs(0),
        }
    }
}
impl DataComponent for ClockComponent {
    fn get_data(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
impl EventListener for ClockComponent {
    fn notify(&mut self, event: &LogEvent) {
        use ClockEvent as E;
        use ClockState as S;
        // use Component as C;

        // let C::GameClock = &event.component else {
        //     return;
        // };
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
}

#[derive(Debug, Serialize)]
pub struct GameClock {
    game_clock: ClockComponent,
}
impl GameClock {
    pub fn new() -> Self {
        GameClock {
            game_clock: ClockComponent::new(),
        }
    }
}
impl DataComponent for GameClock {
    fn get_data(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
impl EventListener for GameClock {
    fn notify(&mut self, event: &LogEvent) {
        self.game_clock.notify(event);
    }
}
impl ScoreboardComponent for GameClock {}

#[derive(Debug, Serialize)]
pub struct ShotClock {
    shot_clock: ClockComponent,
}
impl ShotClock {
    pub fn new() -> Self {
        ShotClock {
            shot_clock: ClockComponent::new(),
        }
    }
}
impl DataComponent for ShotClock {
    fn get_data(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
impl EventListener for ShotClock {
    fn notify(&mut self, event: &LogEvent) {
        use ClockEvent as E;
        use Component as C;
        match (&event.component, &event.event) {
            (C::GameClock, Event::Clock(E::Start | E::Stop)) => self.shot_clock.notify(&LogEvent {
                component: C::ShotClock,
                ..*event
            }),
            (C::GameClock, Event::Clock(E::Expired)) => {
                self.shot_clock.notify(&LogEvent {
                    component: C::ShotClock,
                    event: Event::Clock(E::Stop),
                    ..*event
                });
                self.shot_clock.notify(&LogEvent {
                    component: C::ShotClock,
                    event: Event::Clock(E::Set(Duration::from_secs(0))),
                    ..*event
                });
            }
            _ => self.shot_clock.notify(event),
        };
    }
}
impl ScoreboardComponent for ShotClock {}

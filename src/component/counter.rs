use std::{
    borrow::BorrowMut,
    cell::RefCell,
    sync::{Arc, Mutex},
    time::Instant,
};

use serde::Serialize;
use serde_json::Map;

use crate::{
    component::Component,
    event::{
        states::{CounterEvent, ToggleEvent},
        Event, EventListener, EventLogger, LogEvent,
    },
    Scoreboard, ScoreboardComponent,
};

use super::DataComponent;

#[derive(Debug, Clone)]
pub struct Counter {
    value: u64,
    name: String,
}
impl Counter {
    pub fn new(name: String) -> Self {
        Counter { value: 0, name }
    }
}
impl DataComponent for Counter {
    fn get_data(&self) -> serde_json::Value {
        let mut map = Map::default();
        map.insert(
            self.name.clone(),
            serde_json::Value::Number(self.value.into()),
        );
        serde_json::Value::Object(map)
    }
}
impl EventListener for Counter {
    fn notify(&mut self, event: &LogEvent) {
        let Event::Counter(counter_event) = &event.event else {
            return;
        };
        use CounterEvent as E;
        self.value = match *counter_event {
            E::Increment => self.value + 1,
            E::Decrement => self.value.saturating_sub(1),
            E::Set(value) => value,
        }
    }
}
impl ScoreboardComponent for Counter {}

#[derive(Debug)]
pub struct TeamFoulCounter<EL: EventLogger> {
    event_logger: Arc<Mutex<EL>>,
    team_fouls: Counter,
}
impl<EL: EventLogger> TeamFoulCounter<EL> {
    pub fn new(event_logger: Arc<Mutex<EL>>, name: String) -> Self {
        TeamFoulCounter {
            event_logger,
            team_fouls: Counter::new(name),
        }
    }
}
impl<EL: EventLogger> DataComponent for TeamFoulCounter<EL> {
    fn get_data(&self) -> serde_json::Value {
        self.team_fouls.get_data()
    }
}
impl<EL: EventLogger> EventListener for TeamFoulCounter<EL> {
    fn notify(&mut self, event: &LogEvent) {
        use CounterEvent as E;
        match &event.event {
            Event::Counter(E::Increment)
                if self.team_fouls.value != 4 && (self.team_fouls.value + 1) % 5 == 0 =>
            {
                self.event_logger.borrow_mut().lock().unwrap().log_event(
                    Component::HomeTeamFoulWarning,
                    Event::ToggleEvent(ToggleEvent::Activate),
                )
            }
            _ => {
            }
        };
        self.team_fouls.notify(event);
    }
}
impl<EL: EventLogger> ScoreboardComponent for TeamFoulCounter<EL> {}

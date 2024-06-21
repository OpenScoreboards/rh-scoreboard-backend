use serde::Serialize;
use serde_json::Map;

use crate::{
    event::{states::CounterEvent, Event, EventListener, LogEvent},
    ScoreboardComponent,
};

use super::DataComponent;

#[derive(Debug, Serialize)]
pub struct Counter {
    value: u64,
    #[serde(skip_serializing)]
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

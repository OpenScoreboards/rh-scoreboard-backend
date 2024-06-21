use std::{collections::HashMap, time::Instant};

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    component::{Component, DataComponent},
    event::{Event, EventListener, EventLogger, LogEvent},
};

pub trait ScoreboardComponent: EventListener + DataComponent {}

#[derive(Default, Debug)]
pub struct Scoreboard {
    event_log: Vec<LogEvent>,
    components: Vec<Box<dyn ScoreboardComponent>>,
    listeners: HashMap<Component, Vec<usize>>,
}
impl Scoreboard {
    pub fn add_component<C: ScoreboardComponent + 'static>(
        &mut self,
        component: C,
        listening: &[Component],
    ) {
        let component_index = self.components.len();
        self.components.push(Box::from(component));
        for listen in listening {
            let list = self.listeners.entry(*listen).or_default();
            list.push(component_index);
        }
    }
}
impl DataComponent for Scoreboard {
    fn get_data(&self) -> serde_json::Value {
        let mut data_map = Map::<String, Value>::default();
        for component in &self.components {
            match component.get_data() {
                Value::Object(map) => data_map.extend(map.into_iter()),
                _ => panic!("Unknown JSON data type"),
            }
        }
        serde_json::Value::Object(data_map)
    }
}
impl EventLogger for Scoreboard {
    fn log_event(&mut self, component: Component, event: Event) {
        let Some(listeners) = self.listeners.get(&component) else {
            eprintln!("There are no listeners for {:?}", component);
            return;
        };
        let log_event = LogEvent {
            timestamp: Instant::now(),
            log_id: Uuid::new_v4(),
            component,
            event,
        };
        for &listener in listeners {
            let Some(listener_component) = self.components.get_mut(listener) else {
                eprintln!("component {listener} doesn't exist");
                continue;
            };
            listener_component.notify(&log_event);
        }
        self.event_log.push(log_event);
        // push to DB
    }
    // fn register_listener(&mut self, component: Component, listener: &'a mut dyn EventListener) {
    //     let list = self.listeners.entry(component).or_default();
    //     list.push(listener);
    // }
}

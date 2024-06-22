use rocket::tokio::sync::broadcast::{Receiver, Sender};
use serde_json::Map;

use crate::{
    component::{Component, GlobalComponent},
    event::{
        handle_data_log,
        states::{CounterEvent, ToggleEvent},
        Event, LogEvent,
    },
};

use super::TeamComponent;

#[derive(Debug, Clone)]
struct InternalCounter {
    value: u64,
    name: String,
}
impl InternalCounter {
    pub fn new(name: String) -> Self {
        InternalCounter { value: 0, name }
    }
    fn get_data(&self) -> serde_json::Value {
        let mut map = Map::default();
        map.insert(
            self.name.clone(),
            serde_json::Value::Number(self.value.into()),
        );
        serde_json::Value::Object(map)
    }
    fn process_event(&mut self, event: &LogEvent) {
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

#[derive(Debug)]
pub struct Counter {
    component: Component,
    send: Sender<LogEvent>,
    receive: Receiver<LogEvent>,
    counter: InternalCounter,
}
impl Counter {
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
            counter: InternalCounter::new(name.into()),
        }
    }
    pub async fn run(mut self) {
        while let Ok(log_event) = self.receive.recv().await {
            if handle_data_log(&log_event, self.component, &self.send, || {
                self.counter.get_data()
            }) {
                continue;
            }
            if log_event.component != self.component {
                continue;
            }
            self.counter.process_event(&log_event);
        }
    }
}
#[derive(Debug)]
pub struct TeamFoulCounter {
    component: Component,
    send: Sender<LogEvent>,
    receive: Receiver<LogEvent>,
    counter: InternalCounter,
}
impl TeamFoulCounter {
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
            counter: InternalCounter::new(name.into()),
        }
    }
    pub async fn run(mut self) {
        while let Ok(log_event) = self.receive.recv().await {
            if handle_data_log(&log_event, self.component, &self.send, || {
                self.counter.get_data()
            }) {
                continue;
            }
            if log_event.component != self.component {
                continue;
            }
            self.counter.process_event(&log_event);

            let target = match self.component {
                Component::Away(_) => Component::Away(TeamComponent::TeamFoulWarning),
                Component::Home(_) => Component::Home(TeamComponent::TeamFoulWarning),
                _ => continue,
            };
            let value = if self.counter.value > 5 && (self.counter.value + 1) % 5 == 0 {
                ToggleEvent::Activate
            } else {
                ToggleEvent::Deactivate
            };
            self.send
                .send(LogEvent::new(target, Event::Toggle(value)))
                .expect("message sent");
        }
    }
}

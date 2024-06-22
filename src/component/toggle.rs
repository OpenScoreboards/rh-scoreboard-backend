
use rocket::{
    tokio::{
        sync::broadcast::{Receiver, Sender},
    },
};
use serde_json::Map;

use crate::{
    component::Component,
    event::{
        handle_data_log,
        states::{ToggleEvent, ToggleState},
        Event, LogEvent,
    },
};

use super::GlobalComponent;

#[derive(Debug, Clone)]
struct InteralToggle {
    state: ToggleState,
    name: String,
}
impl InteralToggle {
    pub fn new(name: String) -> Self {
        InteralToggle {
            state: ToggleState::Inactive,
            name,
        }
    }
    fn get_data(&self) -> serde_json::Value {
        let mut map = Map::default();
        map.insert(
            self.name.clone(),
            serde_json::Value::Bool(matches!(self.state, ToggleState::Active)),
        );
        serde_json::Value::Object(map)
    }
    fn process_event(&mut self, event: &LogEvent) {
        let Event::Toggle(counter_event) = &event.event else {
            return;
        };
        use ToggleEvent as E;
        self.state = match *counter_event {
            E::Activate => ToggleState::Active,
            E::Deactivate => ToggleState::Inactive,
        }
    }
}

#[derive(Debug)]
pub struct Toggle {
    component: Component,
    send: Sender<LogEvent>,
    receive: Receiver<LogEvent>,
    toggle: InteralToggle,
}
impl Toggle {
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
            toggle: InteralToggle::new(name.into()),
        }
    }
    pub async fn run(mut self) {
        while let Ok(log_event) = self.receive.recv().await {
            if handle_data_log(&log_event, self.component, &self.send, || {
                self.toggle.get_data()
            }) {
                continue;
            }
            if log_event.component != self.component {
                continue;
            }
            self.toggle.process_event(&log_event);
        }
    }
}

#[derive(Debug)]
pub struct Siren {
    send: Sender<LogEvent>,
    receive: Receiver<LogEvent>,
    state: InteralToggle,
}
impl Siren {
    pub fn new(send: Sender<LogEvent>, receive: Receiver<LogEvent>) -> Self {
        Self {
            send,
            receive,
            state: InteralToggle::new("siren".into()),
        }
    }
    pub async fn run(mut self) {
        while let Ok(log_event) = self.receive.recv().await {
            if handle_data_log(
                &log_event,
                Component::Global(GlobalComponent::Siren),
                &self.send,
                || self.state.get_data(),
            ) {
                continue;
            }
            if !matches!(
                log_event.component,
                Component::Global(GlobalComponent::Siren)
            ) {
                continue;
            }
            self.state.process_event(&log_event);
        }
    }
}

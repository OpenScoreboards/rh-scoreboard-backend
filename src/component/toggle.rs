use rocket::tokio::{self, sync::broadcast::Sender};
use serde_json::{json, Value};

use crate::{
    component::Component,
    event::{
        states::{ToggleEvent, ToggleState},
        Event, LogEvent, MessageChannel, Shareable,
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
    toggle: Shareable<InteralToggle>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
}
impl Toggle {
    pub fn new(
        event_send: Sender<LogEvent>,
        data_log_send: Sender<Value>,
        component: Component,
        name: &str,
    ) -> Self {
        Self {
            component,
            toggle: InteralToggle::new(name.into()).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
        }
    }
    pub async fn run(mut self) {
        let toggle = self.toggle.clone();
        tokio::spawn(async move {
            loop {
                let Ok(Value::Null) = self.data_channel.recv().await else {
                    continue;
                };
                let toggle = toggle.data.lock().unwrap();
                let _ = self.data_channel.send(json!({
                    &toggle.name: matches!(toggle.state, ToggleState::Active)
                }));
            }
        });
        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if log_event.component != self.component {
                    continue;
                }
                self.toggle.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}

#[derive(Debug)]
pub struct Siren {
    state: Shareable<InteralToggle>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
}
impl Siren {
    pub fn new(event_send: Sender<LogEvent>, data_log_send: Sender<Value>) -> Self {
        Self {
            state: InteralToggle::new("siren".into()).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
        }
    }
    pub async fn run(mut self) {
        let toggle = self.state.clone();
        tokio::spawn(async move {
            loop {
                let Ok(Value::Null) = self.data_channel.recv().await else {
                    continue;
                };
                let toggle = toggle.data.lock().unwrap();
                let _ = self.data_channel.send(json!({
                    &toggle.name: matches!(toggle.state, ToggleState::Active)
                }));
            }
        });
        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if !matches!(
                    log_event.component,
                    Component::Global(GlobalComponent::Siren)
                ) {
                    continue;
                }
                self.state.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}

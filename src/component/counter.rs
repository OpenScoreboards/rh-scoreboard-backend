use rocket::tokio::{self, sync::broadcast::Sender};
use serde_json::{json, Value};

use crate::{
    component::Component,
    event::{
        states::{CounterEvent, ToggleEvent},
        Event, LogEvent, MessageChannel, Shareable,
    },
};

use super::TeamComponent;

#[derive(Debug, Clone)]
struct InternalCounter {
    orig_value: u64,
    value: u64,
    name: String,
}
impl InternalCounter {
    pub fn new(name: String, value: u64) -> Self {
        InternalCounter {
            value,
            orig_value: value,
            name,
        }
    }
    fn process_event(&mut self, event: &LogEvent) {
        if let Event::Reset = &event.event {
            self.value = self.orig_value;
            return;
        }
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
    counter: Shareable<InternalCounter>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
}
impl Counter {
    pub fn new(
        event_send: Sender<LogEvent>,
        data_log_send: Sender<Value>,
        component: Component,
        name: &str,
        value: u64,
    ) -> Self {
        Self {
            component,
            counter: InternalCounter::new(name.into(), value).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
        }
    }
    pub async fn run(mut self) {
        let counter = self.counter.clone();
        tokio::spawn(async move {
            loop {
                let Ok(Value::Null) = self.data_channel.recv().await else {
                    continue;
                };
                let counter = counter.data.lock().unwrap();
                let _ = self
                    .data_channel
                    .send(json!({ counter.name.clone(): counter.value.clone() }));
            }
        });
        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if !self
                    .component
                    .is_event_component_relevant(&log_event.component)
                {
                    continue;
                }
                self.counter.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}
#[derive(Debug)]
pub struct TeamFoulCounter {
    component: Component,
    counter: Shareable<InternalCounter>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
}
impl TeamFoulCounter {
    pub fn new(
        event_send: Sender<LogEvent>,
        data_log_send: Sender<Value>,
        component: Component,
        name: &str,
    ) -> Self {
        Self {
            component,
            counter: InternalCounter::new(name.into(), 0).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
        }
    }
    pub async fn run(mut self) {
        let counter = self.counter.clone();
        tokio::spawn(async move {
            loop {
                let Ok(Value::Null) = self.data_channel.recv().await else {
                    continue;
                };
                let counter = counter.data.lock().unwrap();
                let _ = self
                    .data_channel
                    .send(json!({ &counter.name: counter.value }));
            }
        });
        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if !self
                    .component
                    .is_event_component_relevant(&log_event.component)
                {
                    continue;
                }
                let mut counter = self.counter.data.lock().unwrap();
                counter.process_event(&log_event);

                let target = match self.component {
                    Component::Away(_) => Component::Away(TeamComponent::TeamFoulWarning),
                    Component::Home(_) => Component::Home(TeamComponent::TeamFoulWarning),
                    _ => continue,
                };
                if counter.value > 5 && (counter.value + 1) % 5 == 0 {
                    self.event_channel
                        .send(LogEvent {
                            component: target,
                            event: Event::Toggle(ToggleEvent::Activate),
                            ..log_event
                        })
                        .expect("message sent");
                } else if counter.value > 5 && ((counter.value + 2) % 5 == 0)
                    || ((counter.value) % 5 == 0)
                {
                    self.event_channel
                        .send(LogEvent {
                            component: target,
                            event: Event::Toggle(ToggleEvent::Deactivate),
                            ..log_event
                        })
                        .expect("message sent");
                };
            }
        });
    }
}

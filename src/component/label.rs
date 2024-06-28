use rocket::tokio::{self, sync::broadcast::Sender};
use serde::Serialize;
use serde_json::{json, Value};

use crate::event::{states::LabelEvent, Event, LogEvent, MessageChannel, Shareable};

use super::Component;

#[derive(Debug, Clone, Serialize)]
struct InternalLabel {
    name: String,
    value: String,
}
impl InternalLabel {
    pub fn new(name: String, value: String) -> Self {
        InternalLabel { name, value }
    }
    fn process_event(&mut self, event: &LogEvent) {
        let Event::Label(counter_event) = &event.event else {
            return;
        };
        use LabelEvent as E;
        self.value = match counter_event {
            E::Set(value) => value.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Label {
    component: Component,
    label: Shareable<InternalLabel>,
    event_channel: MessageChannel<LogEvent>,
    data_channel: MessageChannel<Value>,
}
impl Label {
    pub fn new(
        component: Component,
        name: &str,
        event_send: Sender<LogEvent>,
        data_log_send: Sender<Value>,
    ) -> Self {
        Self {
            component,
            label: InternalLabel::new(name.into(), name.into()).into(),
            event_channel: event_send.into(),
            data_channel: data_log_send.into(),
        }
    }
    pub async fn run(mut self) {
        let label = self.label.clone();
        tokio::spawn(async move {
            loop {
                let Ok(Value::Null) = self.data_channel.recv().await else {
                    continue;
                };
                let label = label.data.lock().unwrap();
                let _ = self.data_channel.send(json!({
                    &label.name: label.value,
                }));
            }
        });
        tokio::spawn(async move {
            while let Ok(log_event) = self.event_channel.recv().await {
                if log_event.component != self.component {
                    continue;
                }
                self.label.data.lock().unwrap().process_event(&log_event);
            }
        });
    }
}

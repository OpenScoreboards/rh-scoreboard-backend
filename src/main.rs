#[macro_use]
extern crate rocket;

mod component;
mod event;
// mod scoreboard;
use std::time::Duration;

use component::{
    clock::{start_expiry_watcher, GameClock, GameDependentClock},
    counter::{Counter, TeamFoulCounter},
    label::Label,
    toggle::{Siren, Toggle},
    Component, GlobalComponent, TeamComponent,
};
use event::states::{CounterEvent, LabelEvent, ToggleEvent};
use event::{states::ClockEvent, Event, LogEvent};
use rocket::{
    fairing::{Fairing, Info, Kind},
    futures::SinkExt,
    http::Header,
    tokio::{
        self,
        sync::broadcast::{self, error::RecvError, Sender},
        time::sleep,
    },
    Request, Response, State,
};
use serde_json::{Map, Value};
use ws::Message;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

async fn get_data(sender: &State<Vec<Sender<Value>>>) -> String {
    let mut data_map = Map::<String, Value>::default();
    for channel in sender.iter() {
        let mut recv = channel.subscribe();
        channel.send(Value::Null).expect("data channel closed");

        let message = loop {
            let message = recv.recv().await.expect("data message not received");
            match message {
                Value::Null => continue,
                message => break message,
            }
        };
        let Value::Object(data) = message else {
            panic!("object data not received, go {message:?}");
        };
        data_map.extend(data.into_iter());
    }
    serde_json::Value::Object(data_map).to_string()
}

#[get("/data")]
async fn data(sender: &State<Vec<Sender<Value>>>) -> String {
    get_data(sender).await
}

#[get("/data_stream")]
fn echo_stream<'a>(
    ws: ws::WebSocket,
    event_channel: &'a State<Sender<LogEvent>>,
    data_channels: &'a State<Vec<Sender<Value>>>,
) -> ws::Channel<'a> {
    let mut recv = event_channel.subscribe();
    ws.channel(move |mut stream| {
        Box::pin(async move {
            let mut last = Message::Text(get_data(data_channels).await);
            if let e @ Err(_) = stream.send(last.clone()).await {
                eprintln!("{e:?}");
                return Ok(());
            }
            loop {
                match recv.recv().await {
                    Ok(_) => {}
                    e @ Err(RecvError::Closed) => {
                        eprintln!("{e:?}");
                        break;
                    }
                    _ => continue,
                }
                let data = Message::Text(get_data(data_channels).await);
                if data != last {
                    last = data.clone();
                    if let e @ Err(_) = stream.send(data).await {
                        eprintln!("{e:?}");
                        break;
                    };
                }
            }
            Ok(())
        })
    })
}

// Clocks

#[post("/<target>/<clock_event>?<value>&<ts>&<uuid>")]
fn global_clock_event(
    sender: &State<Sender<LogEvent>>,
    target: GlobalComponent,
    clock_event: ClockEvent,
    value: Option<u64>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    clock_event_handler(
        sender,
        Component::Global(target),
        clock_event,
        value,
        ts,
        uuid,
    );
}
#[post("/home/<target>/<clock_event>?<value>&<ts>&<uuid>")]
fn home_clock_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    clock_event: ClockEvent,
    value: Option<u64>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    clock_event_handler(
        sender,
        Component::Home(target),
        clock_event,
        value,
        ts,
        uuid,
    );
}
#[post("/away/<target>/<clock_event>?<value>&<ts>&<uuid>")]
fn away_clock_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    clock_event: ClockEvent,
    value: Option<u64>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    clock_event_handler(
        sender,
        Component::Away(target),
        clock_event,
        value,
        ts,
        uuid,
    );
}
fn clock_event_handler(
    sender: &State<Sender<LogEvent>>,
    target: Component,
    mut clock_event: ClockEvent,
    value: Option<u64>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    if !target.is_clock() {
        panic!("{target:?} is not a clock component");
    };
    if let (ClockEvent::Set(_), Some(ms)) = (clock_event, value) {
        clock_event = ClockEvent::Set(Duration::from_millis(ms));
    }
    sender
        .send(LogEvent::new(target, Event::Clock(clock_event), ts, uuid))
        .expect("message sent");
}

// Counters

#[post("/<target>/<counter_event>?<value>&<ts>&<uuid>")]
fn global_counter_event(
    sender: &State<Sender<LogEvent>>,
    target: GlobalComponent,
    counter_event: CounterEvent,
    value: Option<u64>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    counter_event_handler(
        sender,
        Component::Global(target),
        counter_event,
        value,
        ts,
        uuid,
    );
}
#[post("/home/<target>/<counter_event>?<value>&<ts>&<uuid>")]
fn home_counter_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    counter_event: CounterEvent,
    value: Option<u64>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    counter_event_handler(
        sender,
        Component::Home(target),
        counter_event,
        value,
        ts,
        uuid,
    );
}
#[post("/away/<target>/<counter_event>?<value>&<ts>&<uuid>")]
fn away_counter_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    counter_event: CounterEvent,
    value: Option<u64>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    counter_event_handler(
        sender,
        Component::Away(target),
        counter_event,
        value,
        ts,
        uuid,
    );
}
fn counter_event_handler(
    sender: &State<Sender<LogEvent>>,
    target: Component,
    mut counter_event: CounterEvent,
    value: Option<u64>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    if !target.is_counter() {
        panic!("{target:?} is not a counter component");
    };
    if let (CounterEvent::Set(_), Some(val)) = (counter_event, value) {
        counter_event = CounterEvent::Set(val);
    }
    sender
        .send(LogEvent::new(
            target,
            Event::Counter(counter_event),
            ts,
            uuid,
        ))
        .expect("message sent");
}

// Toggles

#[post("/<target>/<toggle_event>?<ts>&<uuid>")]
fn global_toggle_event(
    sender: &State<Sender<LogEvent>>,
    target: GlobalComponent,
    toggle_event: ToggleEvent,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    toggle_event_handler(sender, Component::Global(target), toggle_event, ts, uuid);
}
#[post("/home/<target>/<toggle_event>?<ts>&<uuid>")]
fn home_toggle_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    toggle_event: ToggleEvent,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    toggle_event_handler(sender, Component::Home(target), toggle_event, ts, uuid);
}
#[post("/away/<target>/<toggle_event>?<ts>&<uuid>")]
fn away_toggle_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    toggle_event: ToggleEvent,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    toggle_event_handler(sender, Component::Away(target), toggle_event, ts, uuid);
}
fn toggle_event_handler(
    sender: &State<Sender<LogEvent>>,
    target: Component,
    toggle_event: ToggleEvent,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    if !target.is_toggle() {
        panic!("{target:?} is not a clock component");
    };
    sender
        .send(LogEvent::new(target, Event::Toggle(toggle_event), ts, uuid))
        .expect("message sent");
}

// Labels

#[post("/<target>/<label_event>?<value>&<ts>&<uuid>")]
fn global_label_event(
    sender: &State<Sender<LogEvent>>,
    target: GlobalComponent,
    label_event: LabelEvent,
    value: Option<String>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    label_event_handler(
        sender,
        Component::Global(target),
        label_event,
        value,
        ts,
        uuid,
    );
}
#[post("/home/<target>/<label_event>?<value>&<ts>&<uuid>")]
fn home_label_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    label_event: LabelEvent,
    value: Option<String>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    label_event_handler(
        sender,
        Component::Home(target),
        label_event,
        value,
        ts,
        uuid,
    );
}
#[post("/away/<target>/<label_event>?<value>&<ts>&<uuid>")]
fn away_label_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    label_event: LabelEvent,
    value: Option<String>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    label_event_handler(
        sender,
        Component::Away(target),
        label_event,
        value,
        ts,
        uuid,
    );
}
fn label_event_handler(
    sender: &State<Sender<LogEvent>>,
    target: Component,
    mut label_event: LabelEvent,
    value: Option<String>,
    ts: Option<usize>,
    uuid: Option<String>,
) {
    if !target.is_label() {
        panic!("{target:?} is not a label component");
    };
    if let (LabelEvent::Set(_), Some(val)) = (&label_event, value) {
        label_event = LabelEvent::Set(val);
    }
    sender
        .send(LogEvent::new(target, Event::Label(label_event), ts, uuid))
        .expect("message sent");
}

fn create_data_channel<T: Clone>() -> Sender<T> {
    broadcast::channel::<T>(512).0
}

macro_rules! run_components {
    ($send: expr, $data_channels: expr, $($typ: ident { $($arg: expr),* },)* ) => {
        $(
            let data_channel = create_data_channel();
            let component = $typ::new($send.clone(), data_channel.clone(), $($arg),*);
            tokio::spawn(async move { component.run().await });
            $data_channels.push(data_channel);
        )+
    };
}
pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

fn add_components(send: Sender<LogEvent>, data_channels: &mut Vec<Sender<Value>>) {
    use Component as C;
    use GlobalComponent as GC;
    use TeamComponent as TC;

    let game_clock_data = create_data_channel();
    let shot_clock_data = create_data_channel();

    run_components!(
        send,
        data_channels,
        GameClock { game_clock_data.clone() },
        GameDependentClock { C::Global(GC::ShotClock), "shot_clock", shot_clock_data.clone() },
        Siren { },
        Counter { C::Global(GC::Period), "period", 1 },
        Counter { C::Home(TC::Score), "home_score", 0 },
        Counter { C::Away(TC::Score), "away_score", 0 },
        TeamFoulCounter { C::Home(TC::TeamFouls), "home_tf" },
        TeamFoulCounter { C::Away(TC::TeamFouls), "away_tf" },
        Toggle { C::Home(TC::TeamFoulWarning), "home_team_foul_warning" },
        Toggle { C::Away(TC::TeamFoulWarning), "away_team_foul_warning" },
        Toggle { C::Home(TC::TimeOutWarning), "home_team_timeout" },
        Toggle { C::Away(TC::TimeOutWarning), "away_team_timeout" },
        Label { C::Home(TC::TeamName), "home", "Home" },
        Label { C::Away(TC::TeamName), "away", "Away" },
    );

    start_expiry_watcher(
        C::Global(GC::GameClock),
        true,
        send.clone(),
        game_clock_data,
    );
    start_expiry_watcher(
        C::Global(GC::ShotClock),
        false,
        send.clone(),
        shot_clock_data,
    );
}

#[launch]
async fn rocket() -> _ {
    let (send, _) = broadcast::channel::<LogEvent>(2048);
    let mut data_channels = vec![];

    add_components(send.clone(), &mut data_channels);

    rocket::build()
        .attach(CORS)
        .manage(send)
        .manage(data_channels)
        .mount("/", routes![index, data, echo_stream])
        .mount(
            "/clock/",
            routes![global_clock_event, home_clock_event, away_clock_event],
        )
        .mount(
            "/counter/",
            routes![global_counter_event, home_counter_event, away_counter_event],
        )
        .mount(
            "/toggle/",
            routes![global_toggle_event, home_toggle_event, away_toggle_event],
        )
        .mount(
            "/label/",
            routes![global_label_event, home_label_event, away_label_event],
        )
}

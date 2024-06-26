#[macro_use]
extern crate rocket;

mod component;
mod event;
// mod scoreboard;
use std::time::{Duration, Instant};

use component::{
    clock::{start_expiry_watcher, GameClock, GameDependentClock},
    counter::{Counter, TeamFoulCounter},
    toggle::{Siren, Toggle},
    Component, GlobalComponent, TeamComponent,
};
use event::states::{ClockState, CounterEvent, ToggleEvent};
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

#[post("/<target>/<clock_event>?<value>")]
fn global_clock_event(
    sender: &State<Sender<LogEvent>>,
    target: GlobalComponent,
    clock_event: ClockEvent,
    value: Option<u64>,
) {
    clock_event_handler(sender, Component::Global(target), clock_event, value);
}
#[post("/home/<target>/<clock_event>?<value>")]
fn home_clock_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    clock_event: ClockEvent,
    value: Option<u64>,
) {
    clock_event_handler(sender, Component::Home(target), clock_event, value);
}
#[post("/away/<target>/<clock_event>?<value>")]
fn away_clock_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    clock_event: ClockEvent,
    value: Option<u64>,
) {
    clock_event_handler(sender, Component::Away(target), clock_event, value);
}
fn clock_event_handler(
    sender: &State<Sender<LogEvent>>,
    target: Component,
    mut clock_event: ClockEvent,
    value: Option<u64>,
) {
    if !target.is_clock() {
        panic!("{target:?} is not a clock component");
    };
    if let (ClockEvent::Set(_), Some(ms)) = (clock_event, value) {
        clock_event = ClockEvent::Set(Duration::from_millis(ms));
    }
    sender
        .send(LogEvent::new(target, Event::Clock(clock_event)))
        .expect("message sent");
}

// Counters

#[post("/<target>/<counter_event>?<value>")]
fn global_counter_event(
    sender: &State<Sender<LogEvent>>,
    target: GlobalComponent,
    counter_event: CounterEvent,
    value: Option<u64>,
) {
    counter_event_handler(sender, Component::Global(target), counter_event, value);
}
#[post("/home/<target>/<counter_event>?<value>")]
fn home_counter_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    counter_event: CounterEvent,
    value: Option<u64>,
) {
    counter_event_handler(sender, Component::Home(target), counter_event, value);
}
#[post("/away/<target>/<counter_event>?<value>")]
fn away_counter_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    counter_event: CounterEvent,
    value: Option<u64>,
) {
    counter_event_handler(sender, Component::Away(target), counter_event, value);
}
fn counter_event_handler(
    sender: &State<Sender<LogEvent>>,
    target: Component,
    mut counter_event: CounterEvent,
    value: Option<u64>,
) {
    if !target.is_counter() {
        panic!("{target:?} is not a counter component");
    };
    if let (CounterEvent::Set(_), Some(val)) = (counter_event, value) {
        counter_event = CounterEvent::Set(val);
    }
    sender
        .send(LogEvent::new(target, Event::Counter(counter_event)))
        .expect("message sent");
}

// Toggles

#[post("/<target>/<toggle_event>")]
fn global_toggle_event(
    sender: &State<Sender<LogEvent>>,
    target: GlobalComponent,
    toggle_event: ToggleEvent,
) {
    toggle_event_handler(sender, Component::Global(target), toggle_event);
}
#[post("/home/<target>/<toggle_event>")]
fn home_toggle_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    toggle_event: ToggleEvent,
) {
    toggle_event_handler(sender, Component::Home(target), toggle_event);
}
#[post("/away/<target>/<toggle_event>")]
fn away_toggle_event(
    sender: &State<Sender<LogEvent>>,
    target: TeamComponent,
    toggle_event: ToggleEvent,
) {
    toggle_event_handler(sender, Component::Away(target), toggle_event);
}
fn toggle_event_handler(
    sender: &State<Sender<LogEvent>>,
    target: Component,
    toggle_event: ToggleEvent,
) {
    if !target.is_toggle() {
        panic!("{target:?} is not a clock component");
    };
    sender
        .send(LogEvent::new(target, Event::Toggle(toggle_event)))
        .expect("message sent");
}

fn create_data_channel<T: Clone>() -> Sender<T> {
    broadcast::channel::<T>(512).0
}

macro_rules! run_unique_component {
    ($typ: ident, $send: expr, $data_channels: expr $(,)?) => {{
        let data_channel = create_data_channel();
        let component = $typ::new($send.clone(), data_channel.clone());
        tokio::spawn(async move { component.run().await });
        $data_channels.push(data_channel.clone());
        data_channel
    }};
}
macro_rules! run_component {
    ($typ: ident, $component: expr, $name: expr, $send: expr, $data_channels: expr $(,)?) => {
        let data_channel = create_data_channel();
        let component = $typ::new($component, $name, $send.clone(), data_channel.clone());
        tokio::spawn(async move { component.run().await });
        $data_channels.push(data_channel);
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

    let game_clock_data_channel = create_data_channel();
    let game_clock_typed_data_channel = create_data_channel();
    let component = GameClock::new(
        send.clone(),
        game_clock_data_channel.clone(),
        game_clock_typed_data_channel.clone(),
    );
    tokio::spawn(async move { component.run().await });
    data_channels.push(game_clock_data_channel);

    start_expiry_watcher(
        Component::Global(GC::GameClock),
        true,
        send.clone(),
        game_clock_typed_data_channel,
    );
    run_unique_component!(Siren, send, data_channels);

    let shot_clock_data_channel = create_data_channel();
    let shot_clock_typed_data_channel = create_data_channel();
    let component = GameDependentClock::new(
        C::Global(GC::ShotClock),
        "shot_clock",
        send.clone(),
        shot_clock_data_channel.clone(),
        shot_clock_typed_data_channel.clone(),
    );
    start_expiry_watcher(
        Component::Global(GC::ShotClock),
        false,
        send.clone(),
        shot_clock_typed_data_channel,
    );
    tokio::spawn(async move { component.run().await });
    data_channels.push(shot_clock_data_channel);
    run_component!(
        Counter,
        C::Home(TC::Score),
        "home_score",
        send,
        data_channels,
    );
    run_component!(
        Counter,
        C::Away(TC::Score),
        "away_score",
        send,
        data_channels,
    );

    run_component!(
        TeamFoulCounter,
        C::Home(TC::TeamFouls),
        "home_tf",
        send,
        data_channels
    );
    run_component!(
        Toggle,
        C::Home(TC::TeamFoulWarning),
        "home_team_foul_warning",
        send,
        data_channels,
    );
    run_component!(
        TeamFoulCounter,
        C::Away(TC::TeamFouls),
        "away_tf",
        send,
        data_channels,
    );

    run_component!(
        Toggle,
        C::Away(TC::TeamFoulWarning),
        "away_team_foul_warning",
        send,
        data_channels,
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
}

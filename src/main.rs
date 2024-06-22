#[macro_use]
extern crate rocket;

mod component;
mod event;
// mod scoreboard;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use component::{
    clock::{ClockComponent, GameClock, GameDependentClock},
    counter::{Counter, TeamFoulCounter},
    toggle::{Siren, Toggle},
    Component, GlobalComponent, TeamComponent,
};
use event::states::{ClockState, CounterEvent, ToggleEvent};
use event::{states::ClockEvent, Event, LogEvent};
use rocket::{
    fairing::{Fairing, Info, Kind},
    futures::{SinkExt, StreamExt},
    http::Header,
    tokio::{
        self,
        sync::broadcast::{self, Sender},
        time::sleep,
    },
    Request, Response, State,
};
use serde_json::{Map, Value};
use uuid::Uuid;
use ws::Message;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

async fn get_data(sender: &Sender<LogEvent>) -> String {
    let mut recv = sender.subscribe();
    sender
        .send(LogEvent::new(
            Component::Global(GlobalComponent::GameClock),
            Event::DataLog(Value::Null),
        ))
        .unwrap();
    let mut data_map = Map::<String, Value>::default();
    let mut components_received = vec![];
    while components_received.len() < DATA_OBJ_COUNT {
        let Ok(log_event) = recv.recv().await else {
            continue;
        };
        if !components_received.contains(&log_event.component) {
            if let Event::DataLog(Value::Object(map)) = log_event.event {
                components_received.push(log_event.component);
                data_map.extend(map.into_iter())
            }
        }
    }
    serde_json::Value::Object(data_map).to_string()
}

#[get("/data")]
async fn data(sender: &State<Sender<LogEvent>>) -> String {
    get_data(sender).await
}

#[get("/data_stream")]
fn echo_stream(ws: ws::WebSocket, sender: &State<Sender<LogEvent>>) -> ws::Channel {
    let mut recv = sender.subscribe();
    // let b = ws.broadcaster();
    ws.channel(move |mut stream| {
        Box::pin(async move {
            let mut last = Message::Text("".into());
            loop {
                let Ok(message) = recv.recv().await else {
                    continue;
                };
                if matches!(message.event, Event::DataLog(_)) {
                    continue;
                }
                let data = Message::Text(get_data(sender).await);
                if data != last {
                    last = data;
                    stream
                        .send(Message::Text(get_data(sender).await))
                        .await
                        .unwrap();
                }
            }
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

const DATA_OBJ_COUNT: usize = 9;

macro_rules! run_component {
    ($typ: ident, $component: expr, $name: expr, $send: expr $(,)?) => {
        let component = $typ::new($component, $name, $send.clone(), $send.subscribe());
        tokio::spawn(async move { component.run().await });
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

#[launch]
async fn rocket() -> _ {
    use Component as C;
    use GlobalComponent as GC;
    use TeamComponent as TC;
    let (send, receiver) = broadcast::channel::<LogEvent>(32);

    let game_clock = GameClock::new(send.clone(), send.subscribe());
    tokio::spawn(async move { game_clock.run().await });
    let watcher = send.clone();
    tokio::spawn(async move {
        loop {
            let mut recv = watcher.subscribe();
            watcher
                .send(LogEvent::new(
                    Component::Global(GlobalComponent::GameClock),
                    Event::DataLog(Value::Null),
                ))
                .unwrap();
            loop {
                let Ok(LogEvent {
                    component: Component::Global(GlobalComponent::GameClock),
                    event: Event::DataLog(data),
                    ..
                }) = recv.recv().await
                else {
                    continue;
                };
                let Some(game_clock_data) = data.get("game_clock") else {
                    continue;
                };
                let Ok(clock_data): Result<ClockComponent, _> =
                    serde_json::from_value(game_clock_data.clone())
                else {
                    continue;
                };
                if !matches!(clock_data.state, ClockState::Running) {
                    continue;
                }
                let time_elapsed = Instant::now() - clock_data.last_state_change;
                if time_elapsed > clock_data.last_time_remaining {
                    watcher
                        .send(LogEvent::new(
                            Component::Global(GlobalComponent::GameClock),
                            Event::Clock(ClockEvent::Expired),
                        ))
                        .unwrap();
                    watcher
                        .send(LogEvent::new(
                            Component::Global(GlobalComponent::Siren),
                            Event::Toggle(ToggleEvent::Activate),
                        ))
                        .unwrap();
                    sleep(Duration::from_secs(2)).await;
                    watcher
                        .send(LogEvent::new(
                            Component::Global(GlobalComponent::Siren),
                            Event::Toggle(ToggleEvent::Deactivate),
                        ))
                        .unwrap();
                    break;
                }
                sleep(Duration::from_millis(200)).await;
            }
        }
    });
    let siren = Siren::new(send.clone(), send.subscribe());
    tokio::spawn(async move { siren.run().await });

    run_component!(
        GameDependentClock,
        C::Global(GC::ShotClock),
        "shot_clock",
        send,
    );
    run_component!(Counter, C::Home(TC::Score), "home_score", send);
    run_component!(Counter, C::Away(TC::Score), "away_score", send);

    run_component!(TeamFoulCounter, C::Home(TC::TeamFouls), "home_tf", send);
    run_component!(
        Toggle,
        C::Home(TC::TeamFoulWarning),
        "home_team_foul_warning",
        send
    );
    run_component!(TeamFoulCounter, C::Away(TC::TeamFouls), "away_tf", send);
    run_component!(
        Toggle,
        C::Away(TC::TeamFoulWarning),
        "away_team_foul_warning",
        send
    );

    rocket::build()
        .attach(CORS)
        .manage(send)
        .manage(Mutex::new(receiver))
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
    // .mount("/counter/", routes![counter_event])
}

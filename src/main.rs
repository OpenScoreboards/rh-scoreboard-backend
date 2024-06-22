#[macro_use]
extern crate rocket;

mod component;
mod event;
// mod scoreboard;
use std::{sync::Mutex, time::Duration};

use component::{
    clock::{GameClock, GameDependentClock},
    counter::{Counter, TeamFoulCounter},
    toggle::Toggle,
    Component, GlobalComponent, TeamComponent,
};
use event::states::{CounterEvent, ToggleEvent};
use event::{states::ClockEvent, Event, LogEvent};
use rocket::{
    tokio::{
        self,
        sync::broadcast::{self, Sender},
    },
    State,
};
use serde_json::{Map, Value};
use uuid::Uuid;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/data")]
async fn data(sender: &State<Sender<LogEvent>>) -> String {
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

// #[post("/<target>/<counter_event>?<value>")]
// fn counter_event(
//     sender: &State<Sender<LogEvent>>,
//     target: Component,
//     mut counter_event: CounterEvent,
//     value: Option<u64>,
// ) {
//     if !target.is_counter() {
//         panic!("{target:?} is not a counter component");
//     };
//     if let (CounterEvent::Set(_), Some(val)) = (counter_event, value) {
//         counter_event = CounterEvent::Set(val);
//     }
//     sender
//         .send(LogEvent::new(target, Event::Counter(counter_event)))
//         .expect("message sent");
// }

const DATA_OBJ_COUNT: usize = 6;

macro_rules! run_component {
    ($typ: ident, $component: expr, $name: expr, $send: expr $(,)?) => {
        let component = $typ::new($component, $name, $send.clone(), $send.subscribe());
        tokio::spawn(async move { component.run().await });
    };
}

#[launch]
async fn rocket() -> _ {
    use Component as C;
    use GlobalComponent as GC;
    use TeamComponent as TC;
    let (send, receiver) = broadcast::channel::<LogEvent>(32);

    let game_clock = GameClock::new(send.clone(), send.subscribe());
    tokio::spawn(async move { game_clock.run().await });

    run_component!(
        GameDependentClock,
        C::Global(GC::ShotClock),
        "shot_clock",
        send,
    );
    // let shot_clock = GameDependentClock::new(
    //     C::Global(GC::ShotClock),
    //     "shot_clock",
    //     send.clone(),
    //     send.subscribe(),
    // );
    run_component!(Counter, C::Home(TC::Score), "home_score", send);
    // let home_score = Counter::new(
    //     C::Home(TC::Score),
    //     "home_score",
    //     send.clone(),
    //     send.subscribe(),
    // );
    run_component!(Counter, C::Away(TC::Score), "away_score", send);
    // let away_score = Counter::new(
    //     C::Away(TC::Score),
    //     "away_score",
    //     send.clone(),
    //     send.subscribe(),
    // );
    run_component!(TeamFoulCounter, C::Away(TC::TeamFouls), "away_tf", send);
    // let away_tf = TeamFoulCounter::new(
    //     C::Away(TC::TeamFouls),
    //     "away_tf",
    //     send.clone(),
    //     send.subscribe(),
    // );
    run_component!(
        Toggle,
        C::Away(TC::TeamFoulWarning),
        "away_team_foul_warning",
        send
    );

    // let away_tfw = Toggle::new(
    //     C::Away(TC::TeamFoulWarning),
    //     "away_team_foul_warning",
    //     send.clone(),
    //     send.subscribe(),
    // );
    // tokio::spawn(async move { game_clock.run().await });
    // tokio::spawn(async move { shot_clock.run().await });
    // tokio::spawn(async move { home_score.run().await });
    // tokio::spawn(async move { away_score.run().await });
    // tokio::spawn(async move { away_tf.run().await });
    // tokio::spawn(async move { away_tfw.run().await });

    rocket::build()
        .manage(send)
        .manage(Mutex::new(receiver))
        .mount("/", routes![index, data])
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

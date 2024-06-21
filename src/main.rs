#[macro_use]
extern crate rocket;

mod component;
mod event;
mod scoreboard;
use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use component::{
    clock::{GameClock, ShotClock},
    counter::{Counter, TeamFoulCounter},
    Component, DataComponent,
};
use event::{states::ClockEvent, Event, LogEvent};
use event::{states::CounterEvent, EventLogger};
use rocket::{
    tokio::{
        self,
        sync::broadcast::{self, Receiver, Sender},
        task::JoinSet,
    },
    State,
};
use scoreboard::*;
use serde_json::{Map, Value};
use uuid::Uuid;
use Component as SC;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/data")]
async fn data(sender: &State<Sender<LogEvent>>) -> String {
    let mut set = JoinSet::new();
    (0..DATA_OBJ_COUNT).for_each(|_| {
        let mut recv = sender.subscribe();
        set.spawn(async move {
            while let Ok(log_event) = recv.recv().await {
                if let Event::DataLog(data @ Value::Object(_)) = log_event.event {
                    return data;
                }
            }
            Value::Null
        });
    });
    sender
        .send(LogEvent::new(
            Component::GameClock,
            Event::DataLog(Value::Null),
        ))
        .unwrap();
    let mut data_map = Map::<String, Value>::default();
    for _ in 0..DATA_OBJ_COUNT {
        if let Some(Ok(Value::Object(map))) = set.join_next().await {
            data_map.extend(map.into_iter())
        }
    }
    serde_json::Value::Object(data_map).to_string()
}

#[post("/<target>/<clock_event>?<value>")]
fn clock_event(
    // scoreboard: &State<Arc<Mutex<Scoreboard>>>,
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
    // scoreboard
    //     .lock()
    //     .expect("scoreboard mutex")
    //     .log_event(target, Event::Clock(clock_event));
}

// #[post("/<target>/<counter_event>?<value>")]
// fn counter_event(
//     scoreboard: &State<Arc<Mutex<Scoreboard>>>,
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
//     scoreboard
//         .lock()
//         .expect("scoreboard mutex")
//         .log_event(target, Event::Counter(counter_event));
// }

const DATA_OBJ_COUNT: u8 = 2;

#[launch]
async fn rocket() -> _ {
    let (broadcaster, receiver) = broadcast::channel::<LogEvent>(32);
    // let arc_scoreboard = Arc::new(Mutex::new(scoreboard));
    // if let Ok(mut scoreboard) = arc_scoreboard.lock() {
    //     scoreboard.add_component(GameClock::new(), &[SC::GameClock]);
    //     scoreboard.add_component(ShotClock::new(), &[SC::ShotClock, SC::GameClock]);
    //     scoreboard.add_component(Counter::new("home_score".into()), &[SC::HomeScore]);

    //     scoreboard.add_component(
    //         TeamFoulCounter::new(arc_scoreboard.clone(), "home_tf".into()),
    //         &[SC::HomeTeamFouls],
    //     );
    //     scoreboard.add_component(Counter::new("away_score".into()), &[SC::AwayScore]);
    //     scoreboard.add_component(
    //         TeamFoulCounter::new(arc_scoreboard.clone(), "away_tf".into()),
    //         &[SC::AwayTeamFouls],
    //     );
    // } else {
    //     panic!("How did I get here?");
    // }
    let game_clock = GameClock::new((broadcaster.clone(), broadcaster.subscribe()));
    let shot_clock = ShotClock::new((broadcaster.clone(), broadcaster.subscribe()));
    tokio::spawn(async move { game_clock.run().await });
    tokio::spawn(async move { shot_clock.run().await });

    rocket::build()
        .manage(broadcaster)
        .manage(Mutex::new(receiver))
        .mount("/", routes![index, data])
        .mount("/clock/", routes![clock_event])
    // .mount("/counter/", routes![counter_event])
}

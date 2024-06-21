#[macro_use]
extern crate rocket;

mod component;
mod event;
mod scoreboard;
use std::{sync::Mutex, time::Duration};

use component::{
    clock::{GameClock, ShotClock},
    Component, DataComponent,
};
use event::EventLogger;
use event::{states::ClockEvent, Event};
use rocket::State;
use scoreboard::*;
use Component as SC;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/data")]
fn data(scoreboard: &State<Mutex<Scoreboard>>) -> String {
    scoreboard
        .lock()
        .expect("scoreboard mutex")
        .get_data()
        .to_string()
}

#[get("/<target>/<clock_event>?<value>")]
fn component_test(
    scoreboard: &State<Mutex<Scoreboard>>,
    target: Component,
    mut clock_event: ClockEvent,
    value: Option<u64>,
) {
    let (SC::GameClock | SC::ShotClock) = target else {
        panic!("{target:?} is not a clock component");
    };
    if let (ClockEvent::Set(_), Some(ms)) = (clock_event, value) {
        clock_event = ClockEvent::Set(Duration::from_millis(ms));
    }
    eprintln!("event: {clock_event:?}, value: {value:?}");
    scoreboard
        .lock()
        .expect("scoreboard mutex")
        .log_event(target, Event::Clock(clock_event));
}

#[launch]
fn rocket() -> _ {
    let mut scoreboard = Scoreboard::default();
    scoreboard.add_component(GameClock::new(), &[SC::GameClock]);
    scoreboard.add_component(ShotClock::new(), &[SC::ShotClock, SC::GameClock]);

    rocket::build()
        .manage(Mutex::new(scoreboard))
        .mount("/", routes![index, data, component_test])
}

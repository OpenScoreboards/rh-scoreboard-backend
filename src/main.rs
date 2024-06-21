#[macro_use]
extern crate rocket;

mod component;
mod event;
mod scoreboard;
use std::{sync::Mutex, time::Duration};

use component::{
    clock::{GameClock, ShotClock},
    counter::Counter,
    Component, DataComponent,
};
use event::{states::ClockEvent, Event};
use event::{states::CounterEvent, EventLogger};
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

#[post("/<target>/<clock_event>?<value>")]
fn clock_event(
    scoreboard: &State<Mutex<Scoreboard>>,
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
    scoreboard
        .lock()
        .expect("scoreboard mutex")
        .log_event(target, Event::Clock(clock_event));
}

#[post("/<target>/<counter_event>?<value>")]
fn counter_event(
    scoreboard: &State<Mutex<Scoreboard>>,
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
    scoreboard
        .lock()
        .expect("scoreboard mutex")
        .log_event(target, Event::Counter(counter_event));
}

#[launch]
fn rocket() -> _ {
    let mut scoreboard = Scoreboard::default();
    scoreboard.add_component(GameClock::new(), &[SC::GameClock]);
    scoreboard.add_component(ShotClock::new(), &[SC::ShotClock, SC::GameClock]);
    scoreboard.add_component(Counter::new("home_score".into()), &[SC::HomeScore]);
    scoreboard.add_component(Counter::new("home_tf".into()), &[SC::HomeTeamFouls]);
    scoreboard.add_component(Counter::new("away_score".into()), &[SC::AwayScore]);
    scoreboard.add_component(Counter::new("away_tf".into()), &[SC::AwayTeamFouls]);

    rocket::build()
        .manage(Mutex::new(scoreboard))
        .mount("/", routes![index, data])
        .mount("/clock/", routes![clock_event])
        .mount("/counter/", routes![counter_event])
}

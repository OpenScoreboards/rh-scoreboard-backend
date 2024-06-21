#[macro_use]
extern crate rocket;

mod component;
mod event;
mod scoreboard;
use std::sync::Mutex;

use component::{
    clock::{GameClock, ShotClock},
    Component, DataComponent,
};
use rocket::State;
use scoreboard::*;
use Component as SC;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/")]
fn data(scoreboard: &State<Mutex<Scoreboard>>) -> String {
    scoreboard
        .lock()
        .expect("scoreboard mutex")
        .get_data()
        .to_string()
}

#[launch]
fn rocket() -> _ {
    let mut scoreboard = Scoreboard::default();
    scoreboard.add_component(GameClock::new(), &[SC::GameClock]);
    scoreboard.add_component(ShotClock::new(), &[SC::ShotClock, SC::GameClock]);

    rocket::build()
        .manage(Mutex::new(scoreboard))
        .mount("/", routes![index])
        .mount("/data", routes![data])
}

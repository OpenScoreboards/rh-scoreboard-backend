use rocket::request::FromParam;
use strum::{EnumString, ParseError};

pub mod clock;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum Component {
    GameClock,
    ShotClock,
    HomeScore,
    AwayScore,
    // ...
}
impl<'a> FromParam<'a> for Component {
    type Error = ParseError;
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.try_into()
    }
}

pub trait DataComponent: Sync {
    fn get_data(&self) -> serde_json::Value;
}

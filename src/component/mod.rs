use rocket::request::FromParam;
use strum::{EnumString, ParseError};

pub mod clock;
pub mod counter;

macro_rules! generate_components {
    (
        global:
            $(clock: $(- $g_clock_name: ident)*)?
            $(counter: $(- $g_counter_name: ident)*)?
            $(toggle: $(- $g_toggle_name: ident)*)?
        per_team:
            $(clock: $(- $t_clock_name: ident)*)?
            $(counter: $(- $t_counter_name: ident)*)?
            $(toggle: $(- $t_toggle_name: ident)*)?
    ) => {
        paste::paste! {
            #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumString)]
            #[strum(ascii_case_insensitive)]
            pub enum Component {
                $($($g_clock_name),*,)?
                $($($g_counter_name),*,)?
                $($($g_toggle_name),*,)?

                $($([<Home $t_clock_name>]),*,)?
                $($([<Home $t_counter_name>]),*,)?
                $($([<Home $t_toggle_name>]),*,)?
                $($([<Away $t_clock_name>]),*,)?
                $($([<Away $t_counter_name>]),*,)?
                $($([<Away $t_toggle_name>]),*,)?
            }
            impl Component {
                pub fn is_clock(&self) -> bool {
                    matches!(
                        self,
                        $($(Self::$g_clock_name)|*)?
                        $($(Self::[<Home $t_clock_name>] | Self::[<Away $t_clock_name>])|*)?
                    )
                }
                pub fn is_counter(&self) -> bool {
                    matches!(
                        self,
                        $($(Self::$g_counter_name)|*)?
                        $($(Self::[<Home $t_counter_name>] | Self::[<Away $t_counter_name>])|*)?
                    )
                }
                pub fn is_toggle(&self) -> bool {
                    matches!(
                        self,
                        $($(Self::$g_toggle_name)|*)?
                        $($(Self::[<Home $t_toggle_name>] | Self::[<Away $t_toggle_name>])|*)?
                    )
                }
            }
        }
    };
}

generate_components!(
    global:
        clock:
            - GameClock
            - ShotClock
    per_team:
        counter:
            - Score
            - TeamFouls
        toggle:
            - TimeOutWarning
            - TeamFoulWarning
);

// #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumString)]
// #[strum(ascii_case_insensitive)]
// pub enum Component {
//     GameClock,
//     ShotClock,
//     // Home
//     HomeScore,
//     HomeTeamFouls,
//     HomeTimeOut,
//     HomeTeamFoulWarning,
//     // Away
//     AwayScore,
//     AwayTeamFouls,
//     AwayTimeOut,
//     AwayTeamFoulWarning,
//     // ...
// }
// impl Component {
//     pub fn is_clock(&self) -> bool {
//         match self {
//             Self::GameClock | Self::ShotClock => true,
//             _ => false,
//         }
//     }
//     pub fn is_counter(&self) -> bool {
//         match self {
//             Self::HomeScore | Self::AwayScore => true,
//             _ => false,
//         }
//     }
// }
impl<'a> FromParam<'a> for Component {
    type Error = ParseError;
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.try_into()
    }
}

pub trait DataComponent: Sync {
    fn get_data(&self) -> serde_json::Value;
}

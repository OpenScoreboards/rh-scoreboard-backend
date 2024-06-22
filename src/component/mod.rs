use rocket::request::FromParam;
use strum::{EnumString, ParseError};

pub mod clock;
pub mod counter;
pub mod toggle;

macro_rules! generate_components {
    (
        global:
            clock: $(- $g_clock_name: ident)*
            counter: $(- $g_counter_name: ident)*
            toggle: $(- $g_toggle_name: ident)*
        per_team:
            clock: $(- $t_clock_name: ident)*
            counter: $(- $t_counter_name: ident)*
            toggle: $(- $t_toggle_name: ident)*
    ) => {
            #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
            pub enum Component {
                Global(GlobalComponent),
                Home(TeamComponent),
                Away(TeamComponent),
            }
            impl Component {
                pub fn is_clock(&self) -> bool {
                    match self {
                        Component::Global(c) => c.is_clock(),
                        Component::Home(c) | Component::Away(c) => c.is_clock(),
                    }
                }
                pub fn is_counter(&self) -> bool {
                    match self {
                        Component::Global(c) => c.is_counter(),
                        Component::Home(c) | Component::Away(c) => c.is_counter(),
                    }
                }
                pub fn is_toggle(&self) -> bool {
                    match self {
                        Component::Global(c) => c.is_toggle(),
                        Component::Home(c) | Component::Away(c) => c.is_toggle(),
                    }
                }
            }
            #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumString)]
            #[strum(ascii_case_insensitive)]
            pub enum GlobalComponent {
                $($g_clock_name ,)*
                $($g_counter_name ,)*
                $($g_toggle_name ,)*
            }
            impl GlobalComponent {
                pub fn is_clock(&self) -> bool {
                    $(matches!(self, Self::$g_clock_name))||*
                }
                pub fn is_counter(&self) -> bool {
                    $(matches!(self, Self::$g_counter_name))||*
                }
                pub fn is_toggle(&self) -> bool {
                    $(matches!(self, Self::$g_toggle_name))||*
                }
            }
            #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumString)]
            #[strum(ascii_case_insensitive)]
            pub enum TeamComponent {
                $($t_clock_name ,)*
                $($t_counter_name ,)*
                $($t_toggle_name ,)*
            }
            impl TeamComponent {
                pub fn is_clock(&self) -> bool {
                    $(matches!(self, Self::$t_clock_name))||*
                }
                pub fn is_counter(&self) -> bool {
                    $(matches!(self, Self::$t_counter_name))||*
                }
                pub fn is_toggle(&self) -> bool {
                    $(matches!(self, Self::$t_toggle_name))||*
                }
            }
    };
}

generate_components!(
    global:
        clock:
            - GameClock
            - ShotClock
        counter:
            - Period
        toggle:
            - Siren
    per_team:
        clock:
            - InferiorityClock
        counter:
            - Score
            - TeamFouls
        toggle:
            - TimeOutWarning
            - TeamFoulWarning
);

// enum Component {
//     Global(GlobalComponent),
//     Home(TeamComponent),
//     Away(TeamComponent),
// }
// enum TeamComponent {}
// enum GlobalComponent {}

impl<'a> FromParam<'a> for TeamComponent {
    type Error = ParseError;
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.try_into()
    }
}
impl<'a> FromParam<'a> for GlobalComponent {
    type Error = ParseError;
    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param.try_into()
    }
}

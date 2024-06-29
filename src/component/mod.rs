use rocket::request::FromParam;
use strum::{EnumString, ParseError};

pub mod clock;
pub mod counter;
pub mod label;
pub mod toggle;

macro_rules! generate_components {
    (
        global:
            clock: $(- $g_clock_name: ident)*
            counter: $(- $g_counter_name: ident)*
            toggle: $(- $g_toggle_name: ident)*
            label: $(- $g_label_name: ident)*
        per_team:
            clock: $(- $t_clock_name: ident)*
            counter: $(- $t_counter_name: ident)*
            toggle: $(- $t_toggle_name: ident)*
            label: $(- $t_label_name: ident)*
    ) => {
            #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
            pub enum Component {
                All,
                Global(GlobalComponent),
                Home(TeamComponent),
                Away(TeamComponent),
            }
            impl Component {
                pub fn is_clock(&self) -> bool {
                    match self {
                        Component::Global(c) => c.is_clock(),
                        Component::Home(c) | Component::Away(c) => c.is_clock(),
                        _ => false,
                    }
                }
                pub fn is_counter(&self) -> bool {
                    match self {
                        Component::Global(c) => c.is_counter(),
                        Component::Home(c) | Component::Away(c) => c.is_counter(),
                        _ => false,
                    }
                }
                pub fn is_toggle(&self) -> bool {
                    match self {
                        Component::Global(c) => c.is_toggle(),
                        Component::Home(c) | Component::Away(c) => c.is_toggle(),
                        _ => false,
                    }
                }
                pub fn is_label(&self) -> bool {
                    match self {
                        Component::Global(c) => c.is_label(),
                        Component::Home(c) | Component::Away(c) => c.is_label(),
                        _ => false,
                    }
                }
            }
            #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumString)]
            #[strum(ascii_case_insensitive)]
            pub enum GlobalComponent {
                $($g_clock_name ,)*
                $($g_counter_name ,)*
                $($g_toggle_name ,)*
                $($g_label_name ,)*
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
                pub fn is_label(&self) -> bool {
                    $(matches!(self, Self::$g_label_name))||*
                }
            }
            #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumString)]
            #[strum(ascii_case_insensitive)]
            pub enum TeamComponent {
                $($t_clock_name ,)*
                $($t_counter_name ,)*
                $($t_toggle_name ,)*
                $($t_label_name ,)*
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
                pub fn is_label(&self) -> bool {
                    $(matches!(self, Self::$t_label_name))||*
                }
            }
    };
}

generate_components!(
    global:
        clock:
            - GameClock
            - ShotClock
            - StoppageClock
        counter:
            - Period
        toggle:
            - Siren
        label:
            - MatchTitle
    per_team:
        clock:
            - InferiorityClock
        counter:
            - Score
            - TeamFouls
        toggle:
            - TimeOutWarning
            - TeamFoulWarning
        label:
            - TeamName
);

impl Component {
    fn is_event_component_relevant(&self, event_component: &Component) -> bool {
        self == event_component || event_component == &Component::All
    }
}

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

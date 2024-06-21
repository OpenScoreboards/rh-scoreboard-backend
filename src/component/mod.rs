pub mod clock;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Component {
    GameClock,
    ShotClock,
    // ...
}

pub trait DataComponent: Sync {
    fn get_data(&self) -> serde_json::Value;
}

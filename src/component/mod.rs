pub mod clock;
pub mod counter;
pub mod toggle;

use clock::ClockEvent;
use counter::CounterEvent;
use toggle::ToggleEvent;

pub enum Event {
    Clock(ClockEvent),
    Counter(CounterEvent),
    ToggleEvent(ToggleEvent),
}

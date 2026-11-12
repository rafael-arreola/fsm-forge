//! `Start -> Middle`: move into the counting stage.

use fsm_forge::Edge;

use crate::machine::{CounterEvent, CounterMachine, Stage};

pub struct Advance;

impl Edge<CounterMachine> for Advance {
    const EVENT: CounterEvent = CounterEvent::Advance;
    const FROM: &'static [Stage] = &[Stage::Start];
    const TO: Stage = Stage::Middle;
}

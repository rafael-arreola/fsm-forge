//! `Middle -> End`: complete the workflow once the `at_least_ten` rule allows it.

use fsm_forge::Edge;

use crate::machine::{CounterEvent, CounterMachine, Stage};

pub struct Finish;

impl Edge<CounterMachine> for Finish {
    const EVENT: CounterEvent = CounterEvent::Finish;
    const FROM: &'static [Stage] = &[Stage::Middle];
    const TO: Stage = Stage::End;
}

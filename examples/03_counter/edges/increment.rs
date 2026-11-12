//! `Middle -> Middle`: self-loop that increments the counter by 1.

use fsm_forge::{Edge, Step};

use crate::machine::{Counter, CounterEvent, CounterMachine, Stage};

pub struct Increment;

impl Edge<CounterMachine> for Increment {
    const EVENT: CounterEvent = CounterEvent::Increment;
    const FROM: &'static [Stage] = &[Stage::Middle];
    const TO: Stage = Stage::Middle;

    fn apply(&self, _step: &Step<'_, CounterMachine>, counter: &mut Counter) {
        counter.count += 1;
    }
}

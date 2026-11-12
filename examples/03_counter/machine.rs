//! The counter machine: three states in a straight line (`Start -> Middle -> End`) with a
//! self-loop (`Increment`) on `Middle` and a scoped rule requiring `count >= 10` to enter
//! `End`.

use fsm_forge::{BuildErrors, Engine, Machine, Scope};

use crate::edges::{Advance, Finish, Increment};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterEvent {
    Advance,
    Increment,
    Finish,
}

#[derive(Debug, Clone)]
pub struct Counter {
    pub stage: Stage,
    pub count: u32,
}

pub struct CounterMachine;

impl Machine for CounterMachine {
    type State = Stage;
    type Event = CounterEvent;
    type Entity = Counter;
    type Input = ();
    type Effect = &'static str;
    type Error = &'static str;

    fn state(counter: &Counter) -> Stage {
        counter.stage
    }

    fn set_state(counter: &mut Counter, stage: Stage) {
        counter.stage = stage;
    }
}

pub fn counter_engine() -> Result<Engine<CounterMachine>, BuildErrors<CounterMachine>> {
    Engine::<CounterMachine>::builder()
        .edge(Advance)
        .edge(Increment)
        .edge(Finish)
        // Guard every transition entering `End` (self-loops in `Middle` are ignored).
        .check(Scope::entering(&[Stage::End]), "at_least_ten", |step| {
            if step.entity.count >= 10 {
                Ok(())
            } else {
                Err("count must be at least 10 to finish")
            }
        })
        .emit(
            Scope::entering(&[Stage::End]),
            "completed",
            "counter.finished",
        )
        .initial(Stage::Start)
        .require_events([
            CounterEvent::Advance,
            CounterEvent::Increment,
            CounterEvent::Finish,
        ])
        .require_states([Stage::Start, Stage::Middle, Stage::End])
        .build()
}

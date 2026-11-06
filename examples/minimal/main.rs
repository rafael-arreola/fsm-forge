//! The smallest machine: one edge, one event, one execution.
//!
//! Run with `cargo run --example minimal`.

mod edges;
mod machine;

use fsm_forge::Engine;

use crate::edges::StartJob;
use crate::machine::{Job, JobEvent, JobMachine, JobStatus};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::<JobMachine>::builder()
        .edge(StartJob)
        .initial(JobStatus::Idle)
        .build()?;

    let job = Job {
        status: JobStatus::Idle,
    };
    println!("before: {:?}", job.status);

    let outcome = engine.execute(&job, JobEvent::Start, &())?;
    println!(
        "transition: {:?} -> {:?}",
        outcome.transition.from, outcome.transition.to
    );
    println!("effects: {}", outcome.effects.len());

    let job = outcome.entity;
    println!("after: {:?}", job.status);

    Ok(())
}

//! Step 1 — Minimal machine: one `Machine`, one `Edge` (`Idle -> Running`), and `execute`.
//!
//! Run with `cargo run --example 01_minimal`.

mod edges;
mod machine;

use crate::machine::{Job, JobEvent, JobStatus, job_engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = job_engine()?;

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

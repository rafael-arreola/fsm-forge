//! Step 6 — From-any mode (`execute_from_any`): originless edges (`FROM: &[]`),
//! guarding critical transitions against overrides, and auditing overrides.
//!
//! Run with `cargo run --example 06_from_any`.

mod edges;
mod machine;

use crate::machine::{Operator, Pipeline, PipelineEvent, PipelineState, pipeline_engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = pipeline_engine()?;
    let mut pipeline = Pipeline {
        state: PipelineState::Queued,
        halted_reason: None,
    };
    let admin = Operator { is_admin: true };

    // 1. An originless edge (`FROM: &[]`) is never allowed through normal `execute`.
    let err = engine
        .execute(&pipeline, PipelineEvent::EmergencyStop, &admin)
        .expect_err("normal execute must reject FROM: &[]");
    println!("1. normal EmergencyStop: {err}");

    // 2. Rules still run in `execute_from_any`: `ReleaseBuild` explicitly rejects overrides.
    let err = engine
        .execute_from_any(&pipeline, PipelineEvent::Release, &admin)
        .expect_err("Release forbids from_any");
    println!("2. forced Release from Queued: {err}");

    // 3. `EmergencyStop` through `execute_from_any` checks `is_admin`, halts the pipeline,
    //    and triggers the `Scope::when(|s| s.from_any)` audit effect.
    let outcome = engine.execute_from_any(&pipeline, PipelineEvent::EmergencyStop, &admin)?;
    let effects: Vec<_> = outcome.effect_values().copied().collect();
    pipeline = outcome.entity;
    println!(
        "3. admin EmergencyStop: -> {:?} (reason={:?}, effects={effects:?})",
        pipeline.state, pipeline.halted_reason
    );

    Ok(())
}

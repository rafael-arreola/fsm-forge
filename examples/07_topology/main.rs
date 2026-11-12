//! Step 7 — Topology validation at `build()` (`roots`, `initial`, `require_events`,
//! `require_states`, `BuildErrors`) and `to_mermaid()`.
//!
//! Run with `cargo run --example 07_topology`.

mod edges;
mod machine;

use fsm_forge::Engine;

use crate::edges::{ConflictingSubmit, OrphanStep, SubmitDoc};
use crate::machine::{DocEvent, DocMachine, DocStatus, valid_doc_engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Valid multi-root topology and generated Mermaid diagram.
    let engine = valid_doc_engine()?;
    println!("1. valid topology:");
    println!("  initial: {:?}", engine.initial());
    println!("  roots:   {:?}", engine.roots());
    println!("  mermaid:");
    for line in engine.to_mermaid().lines() {
        println!("    {line}");
    }

    // 2. Invalid topology: `build()` reports EVERY structural problem at once.
    println!("\n2. broken topology (all errors reported together):");
    let errors = Engine::<DocMachine>::builder()
        .edge(SubmitDoc)
        .edge(ConflictingSubmit) // Conflict on (Submit, Draft)
        .edge(OrphanStep) // UnreachableState(OrphanA) and UnreachableState(OrphanB)
        .initial(DocStatus::Draft)
        .require_events([DocEvent::UnwiredEvent]) // MissingEvent(UnwiredEvent)
        .require_states([DocStatus::Imported]) // MissingState(Imported)
        .build()
        .expect_err("broken topology must fail at build time");

    for error in &errors {
        println!("  - {error}");
    }

    Ok(())
}

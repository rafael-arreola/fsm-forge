//! Application layer for the topology example.
//!
//! No database and no assertions: the example prints the entry points and the errors
//! that `build()` reports. The integration tests cover the checks.
//!
//! Run with `cargo run --example initial`.

mod edges;
mod machine;

use fsm_forge::Engine;

use crate::edges::{ApproveDocument, MoveOrphan, SubmitDocument};
use crate::machine::{DocumentMachine, DocumentStatus};

type Res = Result<(), Box<dyn std::error::Error>>;

/// `initial()` exposes the first root and the diagram marks it.
fn initial_root() -> Res {
    println!("\n1. initial root");
    let engine = Engine::<DocumentMachine>::builder()
        .edge(SubmitDocument)
        .edge(ApproveDocument)
        .initial(DocumentStatus::Draft)
        .build()?;

    println!("  initial: {:?}", engine.initial());
    println!("  roots: {:?}", engine.roots());
    println!("  diagram:");
    for line in engine.to_mermaid().lines() {
        println!("    {line}");
    }
    Ok(())
}

/// Several roots: `initial()` is the first one.
fn several_roots() -> Res {
    println!("\n2. several roots");
    let engine = Engine::<DocumentMachine>::builder()
        .edge(SubmitDocument)
        .edge(ApproveDocument)
        .roots([DocumentStatus::Draft, DocumentStatus::Published])
        .build()?;

    println!("  initial: {:?}", engine.initial());
    println!("  roots: {:?}", engine.roots());
    Ok(())
}

/// Without roots, reachability has no definition and is not checked.
fn without_roots() -> Res {
    println!("\n3. without roots");
    let engine = Engine::<DocumentMachine>::builder()
        .edge(SubmitDocument)
        .edge(MoveOrphan)
        .build()?;

    println!("  initial: {:?}", engine.initial());
    println!("  roots: {:?}", engine.roots());
    println!(
        "  diagram starts at a root: {}",
        engine.to_mermaid().contains("[*]")
    );
    Ok(())
}

/// `build()` reports states that no root can reach.
fn unreachable_states() -> Res {
    println!("\n4. unreachable states");
    let result = Engine::<DocumentMachine>::builder()
        .edge(SubmitDocument)
        .edge(MoveOrphan)
        .initial(DocumentStatus::Draft)
        .build();

    match result {
        Ok(_) => println!("  the engine was built; the orphan pair is reachable"),
        Err(errors) => {
            println!("  build errors:");
            for error in errors.iter() {
                println!("    {error}");
            }
        }
    }
    Ok(())
}

fn main() -> Res {
    initial_root()?;
    several_roots()?;
    without_roots()?;
    unreachable_states()?;
    println!("\ninitial example completed");
    Ok(())
}

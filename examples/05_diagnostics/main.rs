//! Step 5 — Querying without executing: `available`, `target`, `diagnose`,
//! `available_detailed`, and inspecting accumulated rule rejections.
//!
//! Run with `cargo run --example 05_diagnostics`.

mod edges;
mod machine;

use crate::machine::{DeployEvent, DeployState, Deployment, Request, deploy_engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = deploy_engine()?;
    let mut deployment = Deployment {
        state: DeployState::Staged,
        checks_passed: false,
    };

    // 1. Pure topology queries (O(1), rules are not evaluated).
    println!("1. topology from Staged:");
    println!("  available events: {:?}", engine.available(&deployment));
    println!(
        "  Promote target:   {:?}",
        engine.target(DeployEvent::Promote, DeployState::Staged)
    );

    // 2. Action screen: `available_detailed` evaluates every available event with the
    //    given input and collects ALL failing rules per event without running effects.
    println!("\n2. action screen before CI and approval:");
    let unapproved = Request {
        approved_by_lead: false,
    };
    for (event, verdict) in engine.available_detailed(&deployment, &unapproved) {
        match verdict {
            Ok(transition) => println!("  {event:?}: enabled (-> {:?})", transition.to),
            Err(rejection) => {
                println!("  {event:?}: disabled");
                for applied in rejection.rules().unwrap_or(&[]) {
                    println!("    - {applied}");
                }
            }
        }
    }

    // 3. `diagnose` once requirements are met: confirms `Promote` will succeed and
    //    shows which rules will participate, still without running effects.
    deployment.checks_passed = true;
    let approved = Request {
        approved_by_lead: true,
    };
    let transition = engine.diagnose(&deployment, DeployEvent::Promote, &approved)?;
    println!(
        "\n3. diagnose after CI + approval: {:?} -> {:?} (rules={:?})",
        transition.from, transition.to, transition.rules
    );

    Ok(())
}

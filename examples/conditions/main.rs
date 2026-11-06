//! Conditions and phases: `check`, `apply`, `verify`, rejection, and diagnosis.
//!
//! There is no database here: the example focuses on evaluating the conditions before
//! executing and on how a rejection reports the rule and the phase.
//!
//! Run with `cargo run --example conditions`.

mod edges;
mod machine;

use crate::machine::{Order, OrderEvent, OrderStatus, order_engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = order_engine()?;
    let mut order = Order {
        status: OrderStatus::Pending,
        total: 100,
        paid: 0,
    };

    println!("1. diagnose a partial payment");
    match engine.diagnose(&order, OrderEvent::Pay, &30) {
        Ok(diagnosis) => println!("  accepted: {:?} -> {:?}", diagnosis.from, diagnosis.to),
        Err(rejection) => println!("  rejected: {rejection}"),
    }

    println!("2. available events with a partial payment");
    for (event, verdict) in engine.available_detailed(&order, &30) {
        match verdict {
            Ok(_) => println!("  {event:?}: allowed"),
            Err(rejection) => println!("  {event:?}: rejected -> {rejection}"),
        }
    }

    println!("3. execute with a partial payment");
    match engine.execute(&order, OrderEvent::Pay, &30) {
        Ok(_) => println!("  unexpected success"),
        Err(rejection) => println!("  rejected: {rejection}"),
    }

    println!("4. execute with the exact payment");
    let outcome = engine.execute(&order, OrderEvent::Pay, &100)?;
    println!(
        "  transition: {:?} -> {:?}",
        outcome.transition.from, outcome.transition.to
    );
    println!("  paid: {}", outcome.entity.paid);
    println!(
        "  effects: {:?}",
        outcome.effect_values().collect::<Vec<_>>()
    );
    order = outcome.entity;

    println!("5. ship the order");
    let outcome = engine.execute(&order, OrderEvent::Ship, &0)?;
    println!(
        "  transition: {:?} -> {:?}",
        outcome.transition.from, outcome.transition.to
    );
    println!(
        "  effects: {:?}",
        outcome.effect_values().collect::<Vec<_>>()
    );

    Ok(())
}

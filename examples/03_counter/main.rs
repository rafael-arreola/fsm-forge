//! Step 3 — Self-loop and builder rules: `Start -> Middle -> End`, incrementing on
//! `Middle` until a scoped `check` (`count >= 10`) unlocks `End`.
//!
//! Run with `cargo run --example 03_counter`.

mod edges;
mod machine;

use crate::machine::{Counter, CounterEvent, Stage, counter_engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = counter_engine()?;
    let mut counter = Counter {
        stage: Stage::Start,
        count: 0,
    };

    // 1. Move from Start to Middle.
    let outcome = engine.execute(&counter, CounterEvent::Advance, &())?;
    counter = outcome.entity;
    println!(
        "1. advanced to {:?} (count={})",
        counter.stage, counter.count
    );

    // 2. Try to finish before reaching 10 -> rejected by the scoped rule `at_least_ten`.
    match engine.execute(&counter, CounterEvent::Finish, &()) {
        Ok(_) => println!("2. unexpected success"),
        Err(rejection) => println!("2. finish blocked: {rejection}"),
    }

    // 3. Increment 10 times on the `Middle -> Middle` self-loop.
    for _ in 0..10 {
        counter = engine
            .execute(&counter, CounterEvent::Increment, &())?
            .entity;
    }
    println!(
        "3. incremented in {:?} (count={})",
        counter.stage, counter.count
    );

    // 4. Now `count >= 10`, so `Finish` reaches `End`.
    let outcome = engine.execute(&counter, CounterEvent::Finish, &())?;
    counter = outcome.entity;
    let effects: Vec<_> = outcome.effects.into_iter().map(|fx| fx.effect).collect();
    println!(
        "4. finished at {:?} (count={}, effects={effects:?})",
        counter.stage, counter.count
    );

    Ok(())
}

//! Step 2 — The four phases (`check`, `apply`, `verify`, `effects`) and the
//! persist-then-dispatch pattern.
//!
//! Run with `cargo run --example 02_phases`.

mod edges;
mod machine;

use crate::machine::{Order, OrderEvent, OrderStatus, order_engine};

fn persist(order: &Order) {
    println!(
        "  db: saved status={:?}, paid={}/{}",
        order.status, order.paid, order.total
    );
}

fn dispatch(effect: &'static str) {
    println!("  fx: {effect}");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = order_engine()?;
    let mut order = Order {
        status: OrderStatus::Pending,
        total: 100,
        paid: 0,
    };

    // 1. Partial payment fails in `check` before cloning or mutating.
    let err = engine
        .execute(&order, OrderEvent::Pay, &30)
        .expect_err("underpayment must fail");
    println!("1. underpay (30/100): {err}");

    // 2. Overpayment passes `check`, runs `apply` on a copy, and fails in `verify`.
    //    The original `order` stays untouched (`paid == 0`, `status == Pending`).
    let err = engine
        .execute(&order, OrderEvent::Pay, &150)
        .expect_err("overpayment must fail");
    println!(
        "2. overpay (150/100): {err} (original paid={} still intact)",
        order.paid
    );

    // 3. Exact payment succeeds: save `outcome.entity` first, then run `outcome.effects`.
    println!("3. exact payment (100/100):");
    let outcome = engine.execute(&order, OrderEvent::Pay, &100)?;
    order = outcome.entity;
    persist(&order);
    for applied in outcome.effects {
        dispatch(applied.effect);
    }

    // 4. Ship the paid order.
    println!("4. ship order:");
    let outcome = engine.execute(&order, OrderEvent::Ship, &0)?;
    order = outcome.entity;
    persist(&order);
    for applied in outcome.effects {
        dispatch(applied.effect);
    }

    Ok(())
}

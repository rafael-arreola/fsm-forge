//! Application layer: preload, execute, persist, and dispatch.
//!
//! Run with `cargo run --example invoice`.

mod edges;
mod machine;

use fsm_forge::Engine;

use crate::machine::{Invoice, InvoiceEvent, InvoiceMachine, InvoiceStatus, invoice_engine};

/// Illustrative preload from a database.
fn preload_invoice() -> Invoice {
    println!("db: load invoice 9001");
    Invoice {
        status: InvoiceStatus::Draft,
        total: 100,
        paid: 0,
    }
}

/// Illustrative persistence. Run it after the transition was accepted.
fn persist(invoice: &Invoice) {
    println!(
        "db: save invoice 9001 (status={:?}, paid={}/{})",
        invoice.status, invoice.paid, invoice.total
    );
}

/// Illustrative external work. The engine only describes the effect; the application
/// decides what to do with it.
fn dispatch(event: InvoiceEvent, effect: &'static str) {
    match (event, effect) {
        (InvoiceEvent::Issue, "invoice.issued") => {
            println!("mq: email the invoice to the customer");
        }
        (InvoiceEvent::Pay, "invoice.paid") => {
            println!("mq: send the payment receipt");
        }
        (_, other) => println!("log: unhandled effect {other}"),
    }
}

/// One complete stage: execute, persist, dispatch, and keep the result.
fn run_event(
    engine: &Engine<InvoiceMachine>,
    invoice: &mut Invoice,
    event: InvoiceEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{event:?}");

    let outcome = engine.execute(invoice, event, &())?;
    println!(
        "  {:?} -> {:?}",
        outcome.transition.from, outcome.transition.to
    );
    println!(
        "  effects: {:?}",
        outcome.effect_values().collect::<Vec<_>>()
    );

    persist(&outcome.entity);

    for effect in outcome.effect_values() {
        dispatch(outcome.transition.event, effect);
    }

    *invoice = outcome.entity;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = invoice_engine()?;
    let mut invoice = preload_invoice();

    run_event(&engine, &mut invoice, InvoiceEvent::Issue)?;
    run_event(&engine, &mut invoice, InvoiceEvent::Pay)?;

    println!(
        "\nfinal invoice: {:?}, paid {}/{}",
        invoice.status, invoice.paid, invoice.total
    );
    Ok(())
}

//! Full example: the lifecycle of a support ticket.
//!
//! The database and messaging functions only print. The happy path is the main thread;
//! one rejected condition and one supervisor override are shown at the end.
//!
//! Run with `cargo run --example ticket` or
//! `cargo run --example ticket -- --mermaid`.

mod edges;
mod machine;

use fsm_forge::{Engine, Rejection};

use crate::machine::{
    Actor, Command, Priority, Ticket, TicketEffect, TicketEvent, TicketMachine, ticket_engine,
};

/// Illustrative preload from a database.
fn preload_ticket() -> Ticket {
    println!("db: load ticket");
    Ticket::initial()
}

/// Illustrative persistence. Run it after `execute` accepted the transition.
fn persist(ticket: &Ticket) {
    println!(
        "db: save ticket (status={:?}, folio={:?}, agent={:?})",
        ticket.status, ticket.folio, ticket.agent
    );
}

/// Illustrative external work. The engine only describes the effect.
fn dispatch(effect: TicketEffect) {
    match effect {
        TicketEffect::NotifyRequester(message) => println!("mq: notify requester: {message}"),
        TicketEffect::NotifyAgent(message) => println!("mq: notify agent: {message}"),
        TicketEffect::StartSla => println!("sla: start"),
        TicketEffect::PauseSla => println!("sla: pause"),
        TicketEffect::ResumeSla => println!("sla: resume"),
        TicketEffect::PublishUpdate => println!("mq: publish ticket update"),
        TicketEffect::SendSurvey => println!("mq: send satisfaction survey"),
        TicketEffect::SyncAssignment => println!("dir: sync assignment"),
        TicketEffect::AuditOverride => println!("log: audit supervisor override"),
    }
}

/// One complete stage: execute, persist, dispatch, and keep the result.
fn execute_stage(
    engine: &Engine<TicketMachine>,
    ticket: &mut Ticket,
    event: TicketEvent,
    command: Command,
) -> Result<(), Rejection<TicketMachine>> {
    println!("\n{event:?}");

    let outcome = engine.execute(ticket, event, &command)?;
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
        dispatch(*effect);
    }

    *ticket = outcome.entity;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = ticket_engine()?;

    if std::env::args().any(|argument| argument == "--mermaid") {
        println!("{}", engine.to_mermaid());
        return Ok(());
    }

    // 1. Preload the entity.
    let mut ticket = preload_ticket();

    // 2. Happy path.
    execute_stage(
        &engine,
        &mut ticket,
        TicketEvent::NewDraft,
        Command {
            actor: Actor::Requester,
            description: Some("The shipping panel does not load"),
            ..Command::default()
        },
    )?;

    execute_stage(
        &engine,
        &mut ticket,
        TicketEvent::TicketSubmitted,
        Command {
            actor: Actor::Requester,
            folio: Some(4821),
            ..Command::default()
        },
    )?;

    // 3. One rejected condition: assignment without an agent.
    println!("\ncondition: assignment without an agent");
    match engine.execute(&ticket, TicketEvent::AgentAssigned, &Command::default()) {
        Ok(_) => println!("  unexpected success"),
        Err(rejection) => println!("  rejected: {rejection}"),
    }

    execute_stage(
        &engine,
        &mut ticket,
        TicketEvent::AgentAssigned,
        Command {
            actor: Actor::Agent,
            agent: Some("ana"),
            ..Command::default()
        },
    )?;

    execute_stage(
        &engine,
        &mut ticket,
        TicketEvent::WorkStarted,
        Command {
            actor: Actor::Agent,
            diagnosis: Some("corrupted service cache"),
            ..Command::default()
        },
    )?;

    execute_stage(
        &engine,
        &mut ticket,
        TicketEvent::FixDeployed,
        Command {
            actor: Actor::Agent,
            ..Command::default()
        },
    )?;

    execute_stage(
        &engine,
        &mut ticket,
        TicketEvent::SolutionConfirmed,
        Command {
            actor: Actor::Requester,
            confirmed: true,
            ..Command::default()
        },
    )?;

    println!("\nfinal ticket: {:?}", ticket);

    // 4. Override: a supervisor closes a ticket from Open.
    println!("\noverride: supervisor closes a ticket from Open");
    let mut escalated = preload_ticket();
    execute_stage(
        &engine,
        &mut escalated,
        TicketEvent::DirectTicketCreated,
        Command {
            actor: Actor::Requester,
            folio: Some(4822),
            priority: Some(Priority::High),
            ..Command::default()
        },
    )?;

    let command = Command {
        actor: Actor::Supervisor,
        ..Command::default()
    };
    match engine.execute(&escalated, TicketEvent::SupervisorOverride, &command) {
        Ok(_) => println!("  normal execute: unexpected success"),
        Err(rejection) => println!("  normal execute: {rejection}"),
    }

    let outcome = engine.execute_from_any(&escalated, TicketEvent::SupervisorOverride, &command)?;
    println!(
        "  override: {:?} -> {:?}",
        outcome.transition.from, outcome.transition.to
    );
    println!(
        "  effects: {:?}",
        outcome.effect_values().collect::<Vec<_>>()
    );
    persist(&outcome.entity);
    for effect in outcome.effect_values() {
        dispatch(*effect);
    }
    escalated = outcome.entity;
    println!("\nfinal escalated ticket: {:?}", escalated.status);

    println!("\nticket example completed");
    Ok(())
}

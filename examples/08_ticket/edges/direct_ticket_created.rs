//! `Initial -> Open`: create a ticket directly, with folio and priority.

use fsm_forge::{Edge, Step};

use crate::machine::{
    Priority, Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect,
};

pub struct DirectTicketCreated;

impl Edge<TicketMachine> for DirectTicketCreated {
    const EVENT: TicketEvent = TicketEvent::DirectTicketCreated;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Initial];
    const TO: TicketStatus = TicketStatus::Open;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.folio.is_some(), "missing folio")?;
        expect(step.input.priority.is_some(), "missing priority")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.folio = step.input.folio;
        ticket.priority = step.input.priority.unwrap_or(Priority::Normal);
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("ticket_created")]
    }
}

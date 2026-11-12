//! `Assigned -> InProgress`: start the diagnosis.

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct WorkStarted;

impl Edge<TicketMachine> for WorkStarted {
    const EVENT: TicketEvent = TicketEvent::WorkStarted;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Assigned];
    const TO: TicketStatus = TicketStatus::InProgress;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.agent.is_some(), "no agent assigned")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.diagnosis = step.input.diagnosis.or(ticket.diagnosis);
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("in_progress")]
    }
}

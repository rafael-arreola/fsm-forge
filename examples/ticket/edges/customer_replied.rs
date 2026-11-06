//! `Waiting -> InProgress`: the customer replied, resume the work.

use fsm_forge::{Edge, Rule, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct CustomerReplied;

impl Edge<TicketMachine> for CustomerReplied {
    const EVENT: TicketEvent = TicketEvent::CustomerReplied;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Waiting];
    const TO: TicketStatus = TicketStatus::InProgress;
}

impl Rule<TicketMachine> for CustomerReplied {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.sla_paused, "the ticket is not waiting")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.sla_paused = false;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::ResumeSla]
    }
}

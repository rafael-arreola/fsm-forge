//! `InProgress -> Waiting`: ask the customer for more information.

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct InfoRequested;

impl Edge<TicketMachine> for InfoRequested {
    const EVENT: TicketEvent = TicketEvent::InfoRequested;
    const FROM: &'static [TicketStatus] = &[TicketStatus::InProgress];
    const TO: TicketStatus = TicketStatus::Waiting;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.diagnosis.is_some(), "no active diagnosis")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.sla_paused = true;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::PauseSla]
    }
}

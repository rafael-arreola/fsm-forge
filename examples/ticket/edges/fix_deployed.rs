//! `InProgress -> Resolved`: deploy the fix and record the resolution.

use fsm_forge::{Edge, Rule, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct FixDeployed;

impl Edge<TicketMachine> for FixDeployed {
    const EVENT: TicketEvent = TicketEvent::FixDeployed;
    const FROM: &'static [TicketStatus] = &[TicketStatus::InProgress];
    const TO: TicketStatus = TicketStatus::Resolved;
}

impl Rule<TicketMachine> for FixDeployed {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.diagnosis.is_some(), "no diagnosis to resolve")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.resolution = ticket.diagnosis;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("resolved")]
    }
}

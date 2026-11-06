//! `Resolved -> Closed`: the customer confirms the solution.

use fsm_forge::{Edge, Rule, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct SolutionConfirmed;

impl Edge<TicketMachine> for SolutionConfirmed {
    const EVENT: TicketEvent = TicketEvent::SolutionConfirmed;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Resolved];
    const TO: TicketStatus = TicketStatus::Closed;
}

impl Rule<TicketMachine> for SolutionConfirmed {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.confirmed, "the customer has not confirmed")?;
        expect(step.entity.resolution.is_some(), "no resolution recorded")?;
        expect(!step.from_any, "this transition cannot be overridden")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.closed_reason = Some("confirmed by the customer");
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::SendSurvey]
    }
}

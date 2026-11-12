//! `Open -> Assigned`: assign an agent.

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct AgentAssigned;

impl Edge<TicketMachine> for AgentAssigned {
    const EVENT: TicketEvent = TicketEvent::AgentAssigned;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Open];
    const TO: TicketStatus = TicketStatus::Assigned;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.agent.is_some(), "missing agent")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.agent = step.input.agent;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyAgent("ticket_assigned")]
    }
}

//! `Assigned -> Assigned`: reassign to a different agent.

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct AgentReassigned;

impl Edge<TicketMachine> for AgentReassigned {
    const EVENT: TicketEvent = TicketEvent::AgentReassigned;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Assigned];
    const TO: TicketStatus = TicketStatus::Assigned;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.agent.is_some(), "missing agent")?;
        expect(
            step.input.agent != step.entity.agent,
            "the agent is already assigned",
        )
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.agent = step.input.agent;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyAgent("reassigned")]
    }
}

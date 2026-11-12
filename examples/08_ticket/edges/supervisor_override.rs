//! `Any -> Closed`: supervisor override. No origin, so it is only reachable through
//! `execute_from_any`.

use fsm_forge::{Edge, Step};

use crate::machine::{Actor, Ticket, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct SupervisorOverride;

impl Edge<TicketMachine> for SupervisorOverride {
    const EVENT: TicketEvent = TicketEvent::SupervisorOverride;
    const FROM: &'static [TicketStatus] = &[];
    const TO: TicketStatus = TicketStatus::Closed;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.actor == Actor::Supervisor, "supervisor required")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.closed_reason = Some("overridden by supervisor");
    }
}

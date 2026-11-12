//! `Resolved -> Reopened`: the customer rejects the fix.

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct FixRejected;

impl Edge<TicketMachine> for FixRejected {
    const EVENT: TicketEvent = TicketEvent::FixRejected;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Resolved];
    const TO: TicketStatus = TicketStatus::Reopened;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.reason.is_some(), "missing rejection reason")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.reopen_count += 1;
        ticket.resolution = None;
    }
}

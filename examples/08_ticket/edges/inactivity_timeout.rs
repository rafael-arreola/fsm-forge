//! `Waiting -> Closed`: close after the configured inactivity period.
//!
//! The threshold is a field, not a constant: the same transition can be configured per
//! engine (`InactivityTimeout { hours: 72 }`).

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct InactivityTimeout {
    pub hours: u32,
}

impl Edge<TicketMachine> for InactivityTimeout {
    const EVENT: TicketEvent = TicketEvent::InactivityTimeout;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Waiting];
    const TO: TicketStatus = TicketStatus::Closed;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(
            step.input.inactive_hours >= self.hours,
            "the inactivity period has not elapsed",
        )
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.closed_reason = Some("closed by inactivity");
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("closed_by_inactivity")]
    }
}

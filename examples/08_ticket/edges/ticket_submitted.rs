//! `Draft -> Open`: submit a drafted ticket.

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct TicketSubmitted;

impl Edge<TicketMachine> for TicketSubmitted {
    const EVENT: TicketEvent = TicketEvent::TicketSubmitted;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Draft];
    const TO: TicketStatus = TicketStatus::Open;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(
            step.entity.description.is_some(),
            "draft without a description",
        )?;
        expect(step.input.folio.is_some(), "missing folio")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.folio = step.input.folio;
    }
}

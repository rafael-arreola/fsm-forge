//! `Reopened -> InProgress`: start a new diagnosis after a reopen.

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct NewDiagnosisStarted;

impl Edge<TicketMachine> for NewDiagnosisStarted {
    const EVENT: TicketEvent = TicketEvent::NewDiagnosisStarted;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Reopened];
    const TO: TicketStatus = TicketStatus::InProgress;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.reopen_count > 0, "the ticket was not reopened")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.diagnosis = step.input.diagnosis.or(ticket.diagnosis);
    }
}

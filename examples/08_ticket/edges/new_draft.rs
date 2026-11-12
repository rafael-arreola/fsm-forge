//! `Initial -> Draft`: create a draft with a description.

use fsm_forge::{Edge, Step};

use crate::machine::{Ticket, TicketEffect, TicketEvent, TicketMachine, TicketStatus, expect};

pub struct NewDraft;

impl Edge<TicketMachine> for NewDraft {
    const EVENT: TicketEvent = TicketEvent::NewDraft;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Initial];
    const TO: TicketStatus = TicketStatus::Draft;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(
            step.input.description.is_some(),
            "a draft needs a description",
        )
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.description = step.input.description;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("draft_created")]
    }
}

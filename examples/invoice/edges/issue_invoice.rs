//! `Draft -> Issued`: issue the invoice.

use fsm_forge::{Edge, Rule, Step};

use crate::machine::{Invoice, InvoiceEvent, InvoiceMachine, InvoiceStatus};

pub struct IssueInvoice;

impl Edge<InvoiceMachine> for IssueInvoice {
    const EVENT: InvoiceEvent = InvoiceEvent::Issue;
    const FROM: &'static [InvoiceStatus] = &[InvoiceStatus::Draft];
    const TO: InvoiceStatus = InvoiceStatus::Issued;
}

impl Rule<InvoiceMachine> for IssueInvoice {
    fn effects(&self, _step: &Step<'_, InvoiceMachine>, _invoice: &Invoice) -> Vec<&'static str> {
        vec!["invoice.issued"]
    }
}

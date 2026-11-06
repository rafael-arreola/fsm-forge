//! `Issued -> Paid`: register the payment.

use fsm_forge::{Edge, Rule, Step};

use crate::machine::{Invoice, InvoiceEvent, InvoiceMachine, InvoiceStatus};

pub struct PayInvoice;

impl Edge<InvoiceMachine> for PayInvoice {
    const EVENT: InvoiceEvent = InvoiceEvent::Pay;
    const FROM: &'static [InvoiceStatus] = &[InvoiceStatus::Issued];
    const TO: InvoiceStatus = InvoiceStatus::Paid;
}

impl Rule<InvoiceMachine> for PayInvoice {
    fn apply(&self, _step: &Step<'_, InvoiceMachine>, invoice: &mut Invoice) {
        invoice.paid = invoice.total;
    }

    fn effects(&self, _step: &Step<'_, InvoiceMachine>, _invoice: &Invoice) -> Vec<&'static str> {
        vec!["invoice.paid"]
    }
}

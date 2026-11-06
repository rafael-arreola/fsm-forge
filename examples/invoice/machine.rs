//! The invoice machine: domain types, the `Machine` contract, and the engine.

use fsm_forge::{BuildErrors, Engine, Machine};

use crate::edges::{IssueInvoice, PayInvoice};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvoiceStatus {
    Draft,
    Issued,
    Paid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvoiceEvent {
    Issue,
    Pay,
}

#[derive(Debug, Clone)]
pub struct Invoice {
    pub status: InvoiceStatus,
    pub total: i64,
    pub paid: i64,
}

pub struct InvoiceMachine;

impl Machine for InvoiceMachine {
    type State = InvoiceStatus;
    type Event = InvoiceEvent;
    type Entity = Invoice;
    type Input = ();
    type Effect = &'static str;
    type Error = &'static str;

    fn state(invoice: &Invoice) -> InvoiceStatus {
        invoice.status
    }

    fn set_state(invoice: &mut Invoice, status: InvoiceStatus) {
        invoice.status = status;
    }
}

pub fn invoice_engine() -> Result<Engine<InvoiceMachine>, BuildErrors<InvoiceMachine>> {
    Engine::<InvoiceMachine>::builder()
        .edge(IssueInvoice)
        .edge(PayInvoice)
        .initial(InvoiceStatus::Draft)
        .require_events([InvoiceEvent::Issue, InvoiceEvent::Pay])
        .require_states([
            InvoiceStatus::Draft,
            InvoiceStatus::Issued,
            InvoiceStatus::Paid,
        ])
        .build()
}

//! One type per transition.

mod issue_invoice;
mod pay_invoice;

pub use issue_invoice::IssueInvoice;
pub use pay_invoice::PayInvoice;

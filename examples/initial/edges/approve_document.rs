//! `Reviewed -> Published`: approve the document.

use fsm_forge::{Edge, Rule};

use crate::machine::{DocumentEvent, DocumentMachine, DocumentStatus};

pub struct ApproveDocument;

impl Edge<DocumentMachine> for ApproveDocument {
    const EVENT: DocumentEvent = DocumentEvent::Approve;
    const FROM: &'static [DocumentStatus] = &[DocumentStatus::Reviewed];
    const TO: DocumentStatus = DocumentStatus::Published;
}

impl Rule<DocumentMachine> for ApproveDocument {}

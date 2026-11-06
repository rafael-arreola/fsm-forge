//! `Draft -> Reviewed`: submit the document.

use fsm_forge::{Edge, Rule};

use crate::machine::{DocumentEvent, DocumentMachine, DocumentStatus};

pub struct SubmitDocument;

impl Edge<DocumentMachine> for SubmitDocument {
    const EVENT: DocumentEvent = DocumentEvent::Submit;
    const FROM: &'static [DocumentStatus] = &[DocumentStatus::Draft];
    const TO: DocumentStatus = DocumentStatus::Reviewed;
}

impl Rule<DocumentMachine> for SubmitDocument {}

//! `OrphanDraft -> OrphanReviewed`: a disconnected pair, used to demonstrate
//! `UnreachableState`.

use fsm_forge::{Edge, Rule};

use crate::machine::{DocumentEvent, DocumentMachine, DocumentStatus};

pub struct MoveOrphan;

impl Edge<DocumentMachine> for MoveOrphan {
    const EVENT: DocumentEvent = DocumentEvent::MoveOrphan;
    const FROM: &'static [DocumentStatus] = &[DocumentStatus::OrphanDraft];
    const TO: DocumentStatus = DocumentStatus::OrphanReviewed;
}

impl Rule<DocumentMachine> for MoveOrphan {}

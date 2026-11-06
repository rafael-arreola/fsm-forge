//! The document machine: domain types and the `Machine` contract.

use fsm_forge::Machine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentStatus {
    Draft,
    Reviewed,
    Published,
    OrphanDraft,
    OrphanReviewed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentEvent {
    Submit,
    Approve,
    MoveOrphan,
}

#[derive(Clone)]
pub struct Document {
    pub state: DocumentStatus,
}

pub struct DocumentMachine;

impl Machine for DocumentMachine {
    type State = DocumentStatus;
    type Event = DocumentEvent;
    type Entity = Document;
    type Input = ();
    type Effect = ();
    type Error = String;

    fn state(document: &Document) -> DocumentStatus {
        document.state
    }

    fn set_state(document: &mut Document, state: DocumentStatus) {
        document.state = state;
    }
}

//! The document machine: demonstrates `roots`, `initial`, build-time validation
//! (`BuildErrors`), custom labels (`state_name` / `event_name`), and `to_mermaid()`.

use fsm_forge::{BuildErrors, Engine, Machine};

use crate::edges::{ApproveDoc, PublishImported, SubmitDoc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocStatus {
    Draft,
    Imported,
    Review,
    Published,
    OrphanA,
    OrphanB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocEvent {
    Submit,
    Approve,
    PublishImported,
    OrphanMove,
    UnwiredEvent,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub status: DocStatus,
}

pub struct DocMachine;

impl Machine for DocMachine {
    type State = DocStatus;
    type Event = DocEvent;
    type Entity = Document;
    type Input = ();
    type Effect = ();
    type Error = &'static str;

    fn state(doc: &Document) -> DocStatus {
        doc.status
    }

    fn set_state(doc: &mut Document, status: DocStatus) {
        doc.status = status;
    }

    fn state_name(state: DocStatus) -> String {
        match state {
            DocStatus::Review => "In Review".to_string(),
            other => format!("{other:?}"),
        }
    }
}

pub fn valid_doc_engine() -> Result<Engine<DocMachine>, BuildErrors<DocMachine>> {
    Engine::<DocMachine>::builder()
        .edge(SubmitDoc)
        .edge(ApproveDoc)
        .edge(PublishImported)
        .roots([DocStatus::Draft, DocStatus::Imported])
        .initial(DocStatus::Draft)
        .require_events([
            DocEvent::Submit,
            DocEvent::Approve,
            DocEvent::PublishImported,
        ])
        .require_states([
            DocStatus::Draft,
            DocStatus::Imported,
            DocStatus::Review,
            DocStatus::Published,
        ])
        .build()
}

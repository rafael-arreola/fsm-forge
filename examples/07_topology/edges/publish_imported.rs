//! `Imported -> Published`: publish an imported document directly.

use fsm_forge::Edge;

use crate::machine::{DocEvent, DocMachine, DocStatus};

pub struct PublishImported;

impl Edge<DocMachine> for PublishImported {
    const EVENT: DocEvent = DocEvent::PublishImported;
    const FROM: &'static [DocStatus] = &[DocStatus::Imported];
    const TO: DocStatus = DocStatus::Published;
}

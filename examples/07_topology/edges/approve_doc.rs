//! `Review -> Published`: approve a reviewed document.

use fsm_forge::Edge;

use crate::machine::{DocEvent, DocMachine, DocStatus};

pub struct ApproveDoc;

impl Edge<DocMachine> for ApproveDoc {
    const EVENT: DocEvent = DocEvent::Approve;
    const FROM: &'static [DocStatus] = &[DocStatus::Review];
    const TO: DocStatus = DocStatus::Published;
}

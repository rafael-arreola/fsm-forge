//! `Draft -> Review`: submit a draft document.

use fsm_forge::Edge;

use crate::machine::{DocEvent, DocMachine, DocStatus};

pub struct SubmitDoc;

impl Edge<DocMachine> for SubmitDoc {
    const EVENT: DocEvent = DocEvent::Submit;
    const FROM: &'static [DocStatus] = &[DocStatus::Draft];
    const TO: DocStatus = DocStatus::Review;
}

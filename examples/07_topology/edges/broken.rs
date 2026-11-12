//! Intentionally broken edges used in `main.rs` to showcase `BuildErrors` at startup.

use fsm_forge::Edge;

use crate::machine::{DocEvent, DocMachine, DocStatus};

/// Conflicts with `SubmitDoc` on `(Submit, Draft)`.
pub struct ConflictingSubmit;

impl Edge<DocMachine> for ConflictingSubmit {
    const EVENT: DocEvent = DocEvent::Submit;
    const FROM: &'static [DocStatus] = &[DocStatus::Draft];
    const TO: DocStatus = DocStatus::Published;
}

/// Disconnected transition unreachable from `Draft`.
pub struct OrphanStep;

impl Edge<DocMachine> for OrphanStep {
    const EVENT: DocEvent = DocEvent::OrphanMove;
    const FROM: &'static [DocStatus] = &[DocStatus::OrphanA];
    const TO: DocStatus = DocStatus::OrphanB;
}

//! `Queued -> Running`: start the build normally.

use fsm_forge::Edge;

use crate::machine::{PipelineEvent, PipelineMachine, PipelineState};

pub struct StartBuild;

impl Edge<PipelineMachine> for StartBuild {
    const EVENT: PipelineEvent = PipelineEvent::Start;
    const FROM: &'static [PipelineState] = &[PipelineState::Queued];
    const TO: PipelineState = PipelineState::Running;
}

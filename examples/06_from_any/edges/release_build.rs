//! `Running -> Released`: critical transition that explicitly forbids `from_any` overrides.

use fsm_forge::{Edge, Step};

use crate::machine::{PipelineEvent, PipelineMachine, PipelineState};

pub struct ReleaseBuild;

impl Edge<PipelineMachine> for ReleaseBuild {
    const EVENT: PipelineEvent = PipelineEvent::Release;
    const FROM: &'static [PipelineState] = &[PipelineState::Running];
    const TO: PipelineState = PipelineState::Released;

    fn check(&self, step: &Step<'_, PipelineMachine>) -> Result<(), &'static str> {
        if !step.from_any {
            Ok(())
        } else {
            Err("Release cannot be forced via execute_from_any")
        }
    }
}

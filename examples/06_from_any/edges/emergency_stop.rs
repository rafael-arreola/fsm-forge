//! `Any -> Halted`: originless edge (`FROM: &[]`), reachable ONLY via `execute_from_any`.

use fsm_forge::{Edge, Step};

use crate::machine::{Pipeline, PipelineEvent, PipelineMachine, PipelineState};

pub struct EmergencyStop;

impl Edge<PipelineMachine> for EmergencyStop {
    const EVENT: PipelineEvent = PipelineEvent::EmergencyStop;
    const FROM: &'static [PipelineState] = &[];
    const TO: PipelineState = PipelineState::Halted;

    fn check(&self, step: &Step<'_, PipelineMachine>) -> Result<(), &'static str> {
        if step.input.is_admin {
            Ok(())
        } else {
            Err("admin privileges required for emergency stop")
        }
    }

    fn apply(&self, _step: &Step<'_, PipelineMachine>, pipeline: &mut Pipeline) {
        pipeline.halted_reason = Some("halted by admin");
    }
}

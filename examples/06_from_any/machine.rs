//! The pipeline machine: demonstrates `execute_from_any`, originless edges (`FROM: &[]`),
//! blocking overrides on critical edges (`!step.from_any`), and auditing overrides.

use fsm_forge::{BuildErrors, Engine, Machine, Scope};

use crate::edges::{EmergencyStop, ReleaseBuild, StartBuild};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineState {
    Queued,
    Running,
    Released,
    Halted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineEvent {
    Start,
    Release,
    EmergencyStop,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub state: PipelineState,
    pub halted_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Operator {
    pub is_admin: bool,
}

pub struct PipelineMachine;

impl Machine for PipelineMachine {
    type State = PipelineState;
    type Event = PipelineEvent;
    type Entity = Pipeline;
    type Input = Operator;
    type Effect = &'static str;
    type Error = &'static str;

    fn state(pipeline: &Pipeline) -> PipelineState {
        pipeline.state
    }

    fn set_state(pipeline: &mut Pipeline, state: PipelineState) {
        pipeline.state = state;
    }
}

pub fn pipeline_engine() -> Result<Engine<PipelineMachine>, BuildErrors<PipelineMachine>> {
    Engine::<PipelineMachine>::builder()
        .edge(StartBuild)
        .edge(ReleaseBuild)
        .edge(EmergencyStop)
        // Automatically emit an audit effect whenever a transition runs via `execute_from_any`.
        .emit(
            Scope::when(|step| step.from_any),
            "audit_override",
            "audit.from_any_override",
        )
        .initial(PipelineState::Queued)
        .require_events([
            PipelineEvent::Start,
            PipelineEvent::Release,
            PipelineEvent::EmergencyStop,
        ])
        .require_states([
            PipelineState::Queued,
            PipelineState::Running,
            PipelineState::Released,
            PipelineState::Halted,
        ])
        .build()
}

//! The deployment machine: used to query `available`, `target`, `diagnose`, and
//! `available_detailed` without executing effects.

use fsm_forge::{BuildErrors, Engine, Machine, Scope};

use crate::edges::{CancelDeploy, PromoteDeploy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeployState {
    Staged,
    Live,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeployEvent {
    Promote,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct Deployment {
    pub state: DeployState,
    pub checks_passed: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Request {
    pub approved_by_lead: bool,
}

pub struct DeployMachine;

impl Machine for DeployMachine {
    type State = DeployState;
    type Event = DeployEvent;
    type Entity = Deployment;
    type Input = Request;
    type Effect = &'static str;
    type Error = &'static str;

    fn state(deployment: &Deployment) -> DeployState {
        deployment.state
    }

    fn set_state(deployment: &mut Deployment, state: DeployState) {
        deployment.state = state;
    }
}

pub fn deploy_engine() -> Result<Engine<DeployMachine>, BuildErrors<DeployMachine>> {
    Engine::<DeployMachine>::builder()
        .edge(PromoteDeploy)
        .edge(CancelDeploy)
        .check(
            Scope::entering(&[DeployState::Live]),
            "lead_approval",
            |step| {
                if step.input.approved_by_lead {
                    Ok(())
                } else {
                    Err("release lead approval is required")
                }
            },
        )
        .initial(DeployState::Staged)
        .require_events([DeployEvent::Promote, DeployEvent::Cancel])
        .require_states([
            DeployState::Staged,
            DeployState::Live,
            DeployState::Cancelled,
        ])
        .build()
}

//! `Staged -> Live`: promote only when CI checks have passed.

use fsm_forge::{Edge, Step};

use crate::machine::{DeployEvent, DeployMachine, DeployState, Deployment};

pub struct PromoteDeploy;

impl Edge<DeployMachine> for PromoteDeploy {
    const EVENT: DeployEvent = DeployEvent::Promote;
    const FROM: &'static [DeployState] = &[DeployState::Staged];
    const TO: DeployState = DeployState::Live;

    fn check(&self, step: &Step<'_, DeployMachine>) -> Result<(), &'static str> {
        if step.entity.checks_passed {
            Ok(())
        } else {
            Err("CI checks have not passed")
        }
    }

    fn effects(&self, _step: &Step<'_, DeployMachine>, _next: &Deployment) -> Vec<&'static str> {
        vec!["trigger_production_rollout"]
    }
}

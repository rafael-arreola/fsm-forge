//! `Staged -> Cancelled`: abort a staged deployment at any time.

use fsm_forge::Edge;

use crate::machine::{DeployEvent, DeployMachine, DeployState};

pub struct CancelDeploy;

impl Edge<DeployMachine> for CancelDeploy {
    const EVENT: DeployEvent = DeployEvent::Cancel;
    const FROM: &'static [DeployState] = &[DeployState::Staged];
    const TO: DeployState = DeployState::Cancelled;
}

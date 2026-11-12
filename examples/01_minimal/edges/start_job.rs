//! `Idle -> Running`: the only transition.

use fsm_forge::Edge;

use crate::machine::{JobEvent, JobMachine, JobStatus};

pub struct StartJob;

impl Edge<JobMachine> for StartJob {
    const EVENT: JobEvent = JobEvent::Start;
    const FROM: &'static [JobStatus] = &[JobStatus::Idle];
    const TO: JobStatus = JobStatus::Running;
}

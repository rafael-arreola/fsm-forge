//! The job machine: the smallest possible domain.

use fsm_forge::Machine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobStatus {
    Idle,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobEvent {
    Start,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub status: JobStatus,
}

pub struct JobMachine;

impl Machine for JobMachine {
    type State = JobStatus;
    type Event = JobEvent;
    type Entity = Job;
    type Input = ();
    type Effect = ();
    type Error = &'static str;

    fn state(job: &Job) -> JobStatus {
        job.status
    }

    fn set_state(job: &mut Job, status: JobStatus) {
        job.status = status;
    }
}

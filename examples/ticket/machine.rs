//! The ticket machine: domain types, the `Machine` contract, and the engine.

use fsm_forge::{BuildErrors, Engine, Machine, Scope, emit};

use crate::edges::{
    AgentAssigned, AgentReassigned, CustomerReplied, DirectTicketCreated, FixDeployed, FixRejected,
    InactivityTimeout, InfoRequested, NewDiagnosisStarted, NewDraft, SolutionConfirmed,
    SupervisorOverride, TicketSubmitted, WorkStarted,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TicketStatus {
    Initial,
    Draft,
    Open,
    Assigned,
    InProgress,
    Waiting,
    Resolved,
    Reopened,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TicketEvent {
    NewDraft,
    DirectTicketCreated,
    TicketSubmitted,
    AgentAssigned,
    AgentReassigned,
    WorkStarted,
    InfoRequested,
    CustomerReplied,
    FixDeployed,
    FixRejected,
    NewDiagnosisStarted,
    SolutionConfirmed,
    InactivityTimeout,
    SupervisorOverride,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Priority {
    #[default]
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Actor {
    #[default]
    Requester,
    Agent,
    Supervisor,
}

/// Data carried by the event. The service resolves it and passes it in.
#[derive(Debug, Clone, Copy, Default)]
pub struct Command {
    pub actor: Actor,
    pub description: Option<&'static str>,
    pub folio: Option<u32>,
    pub agent: Option<&'static str>,
    pub priority: Option<Priority>,
    pub diagnosis: Option<&'static str>,
    pub reason: Option<&'static str>,
    pub confirmed: bool,
    pub inactive_hours: u32,
}

/// The record the engine mutates.
#[derive(Debug, Clone)]
pub struct Ticket {
    pub status: TicketStatus,
    pub folio: Option<u32>,
    pub priority: Priority,
    pub description: Option<&'static str>,
    pub agent: Option<&'static str>,
    pub diagnosis: Option<&'static str>,
    pub resolution: Option<&'static str>,
    pub reopen_count: u32,
    pub sla_paused: bool,
    pub closed_reason: Option<&'static str>,
}

impl Ticket {
    pub fn initial() -> Self {
        Self {
            status: TicketStatus::Initial,
            folio: None,
            priority: Priority::Normal,
            description: None,
            agent: None,
            diagnosis: None,
            resolution: None,
            reopen_count: 0,
            sla_paused: false,
            closed_reason: None,
        }
    }
}

/// What the outside world must run after the state is saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketEffect {
    NotifyRequester(&'static str),
    NotifyAgent(&'static str),
    StartSla,
    PauseSla,
    ResumeSla,
    PublishUpdate,
    SendSurvey,
    SyncAssignment,
    AuditOverride,
}

pub struct TicketMachine;

impl Machine for TicketMachine {
    type State = TicketStatus;
    type Event = TicketEvent;
    type Entity = Ticket;
    type Input = Command;
    type Effect = TicketEffect;
    type Error = &'static str;

    fn state(ticket: &Ticket) -> TicketStatus {
        ticket.status
    }

    fn set_state(ticket: &mut Ticket, status: TicketStatus) {
        ticket.status = status;
    }
}

/// Small helper shared by the edge checks.
pub fn expect(condition: bool, reason: &'static str) -> Result<(), &'static str> {
    if condition { Ok(()) } else { Err(reason) }
}

pub fn ticket_engine() -> Result<Engine<TicketMachine>, BuildErrors<TicketMachine>> {
    Engine::<TicketMachine>::builder()
        // Creation (1000)
        .edge(NewDraft)
        .edge(DirectTicketCreated)
        .edge(TicketSubmitted)
        // Triage and diagnosis (2000)
        .edge(AgentAssigned)
        .edge(AgentReassigned)
        .edge(WorkStarted)
        .edge(InfoRequested)
        .edge(CustomerReplied)
        // Resolution and validation (3000)
        .edge(FixDeployed)
        .edge(FixRejected)
        .edge(NewDiagnosisStarted)
        // Closure (4000)
        .edge(SolutionConfirmed)
        .edge(InactivityTimeout { hours: 72 })
        // From-any-only operation: absent from the diagram
        .edge(SupervisorOverride)
        // Global rules by scope
        .rule(
            Scope::always(),
            emit("publish_update", TicketEffect::PublishUpdate),
        )
        .rule(
            Scope::leaving(&[TicketStatus::Draft]),
            emit(
                "notify_submission",
                TicketEffect::NotifyRequester("ticket_submitted"),
            ),
        )
        .rule(
            Scope::entering(&[TicketStatus::Open]),
            emit("start_sla", TicketEffect::StartSla),
        )
        .rule(
            Scope::entering(&[TicketStatus::Reopened]),
            emit(
                "alert_recurrence",
                TicketEffect::NotifyRequester("recurrence"),
            ),
        )
        .rule(
            Scope::when(|step| step.from == TicketStatus::Assigned),
            emit("sync_assignment", TicketEffect::SyncAssignment),
        )
        .rule(
            Scope::on(&[TicketEvent::FixRejected]),
            emit(
                "notify_rejection",
                TicketEffect::NotifyRequester("fix_rejected"),
            ),
        )
        .rule(
            Scope::when(|step| step.from_any),
            emit("override_audit", TicketEffect::AuditOverride),
        )
        .initial(TicketStatus::Initial)
        .require_events([
            TicketEvent::NewDraft,
            TicketEvent::DirectTicketCreated,
            TicketEvent::TicketSubmitted,
            TicketEvent::AgentAssigned,
            TicketEvent::AgentReassigned,
            TicketEvent::WorkStarted,
            TicketEvent::InfoRequested,
            TicketEvent::CustomerReplied,
            TicketEvent::FixDeployed,
            TicketEvent::FixRejected,
            TicketEvent::NewDiagnosisStarted,
            TicketEvent::SolutionConfirmed,
            TicketEvent::InactivityTimeout,
            TicketEvent::SupervisorOverride,
        ])
        .require_states([
            TicketStatus::Initial,
            TicketStatus::Draft,
            TicketStatus::Open,
            TicketStatus::Assigned,
            TicketStatus::InProgress,
            TicketStatus::Waiting,
            TicketStatus::Resolved,
            TicketStatus::Reopened,
            TicketStatus::Closed,
        ])
        .build()
}

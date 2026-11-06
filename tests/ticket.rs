//! Advanced lifecycle: support ticket with scopes, reopens, timeout, and a supervisor
//! override.

use fsm_forge::{BuildErrors, Edge, Engine, Machine, Outcome, Rejection, Rule, Scope, Step, emit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Status {
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
enum Event {
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
enum Priority {
    #[default]
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Actor {
    #[default]
    Requester,
    Agent,
    Supervisor,
}

#[derive(Debug, Clone, Copy, Default)]
struct Command {
    actor: Actor,
    description: Option<&'static str>,
    folio: Option<u32>,
    agent: Option<&'static str>,
    priority: Option<Priority>,
    diagnosis: Option<&'static str>,
    reason: Option<&'static str>,
    confirmed: bool,
    inactive_hours: u32,
}

#[derive(Debug, Clone)]
struct Ticket {
    status: Status,
    folio: Option<u32>,
    priority: Priority,
    description: Option<&'static str>,
    agent: Option<&'static str>,
    diagnosis: Option<&'static str>,
    resolution: Option<&'static str>,
    reopen_count: u32,
    sla_paused: bool,
    closed_reason: Option<&'static str>,
}

impl Ticket {
    fn initial() -> Self {
        Self {
            status: Status::Initial,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TicketEffect {
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

/// Ticket machine: full lifecycle with scopes, reopens, timeout, and supervisor
/// override.
///
/// ```mermaid
/// stateDiagram-v2
///     state "Initial" as Initial
///     state "Draft" as Draft
///     state "Open" as Open
///     state "Assigned" as Assigned
///     state "InProgress" as InProgress
///     state "Waiting" as Waiting
///     state "Resolved" as Resolved
///     state "Reopened" as Reopened
///     state "Closed" as Closed
///     [*] --> Initial
///     Initial --> Draft : NewDraft
///     Initial --> Open : DirectTicketCreated
///     Draft --> Open : TicketSubmitted
///     Open --> Assigned : AgentAssigned
///     Assigned --> Assigned : AgentReassigned
///     Assigned --> InProgress : WorkStarted
///     InProgress --> Waiting : InfoRequested
///     Waiting --> InProgress : CustomerReplied
///     InProgress --> Resolved : FixDeployed
///     Resolved --> Reopened : FixRejected
///     Reopened --> InProgress : NewDiagnosisStarted
///     Resolved --> Closed : SolutionConfirmed
///     Waiting --> Closed : InactivityTimeout
///     note right of Closed : SupervisorOverride
/// ```
struct TicketMachine;

impl Machine for TicketMachine {
    type State = Status;
    type Event = Event;
    type Entity = Ticket;
    type Input = Command;
    type Effect = TicketEffect;
    type Error = &'static str;

    fn state(ticket: &Ticket) -> Status {
        ticket.status
    }

    fn set_state(ticket: &mut Ticket, status: Status) {
        ticket.status = status;
    }
}

fn expect(condition: bool, reason: &'static str) -> Result<(), &'static str> {
    if condition { Ok(()) } else { Err(reason) }
}

// ─────────────────────────────────────────────────────────────────────────────
// Edges
// ─────────────────────────────────────────────────────────────────────────────

struct NewDraft;

impl Edge<TicketMachine> for NewDraft {
    const EVENT: Event = Event::NewDraft;
    const FROM: &'static [Status] = &[Status::Initial];
    const TO: Status = Status::Draft;
}

impl Rule<TicketMachine> for NewDraft {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(
            step.input.description.is_some(),
            "a draft needs a description",
        )
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.description = step.input.description;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("draft_created")]
    }
}

struct DirectTicketCreated;

impl Edge<TicketMachine> for DirectTicketCreated {
    const EVENT: Event = Event::DirectTicketCreated;
    const FROM: &'static [Status] = &[Status::Initial];
    const TO: Status = Status::Open;
}

impl Rule<TicketMachine> for DirectTicketCreated {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.folio.is_some(), "missing folio")?;
        expect(step.input.priority.is_some(), "missing priority")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.folio = step.input.folio;
        ticket.priority = step.input.priority.unwrap_or(Priority::Normal);
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("ticket_created")]
    }
}

struct TicketSubmitted;

impl Edge<TicketMachine> for TicketSubmitted {
    const EVENT: Event = Event::TicketSubmitted;
    const FROM: &'static [Status] = &[Status::Draft];
    const TO: Status = Status::Open;
}

impl Rule<TicketMachine> for TicketSubmitted {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(
            step.entity.description.is_some(),
            "draft without a description",
        )?;
        expect(step.input.folio.is_some(), "missing folio")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.folio = step.input.folio;
    }
}

struct AgentAssigned;

impl Edge<TicketMachine> for AgentAssigned {
    const EVENT: Event = Event::AgentAssigned;
    const FROM: &'static [Status] = &[Status::Open];
    const TO: Status = Status::Assigned;
}

impl Rule<TicketMachine> for AgentAssigned {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.agent.is_some(), "missing agent")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.agent = step.input.agent;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyAgent("ticket_assigned")]
    }
}

struct AgentReassigned;

impl Edge<TicketMachine> for AgentReassigned {
    const EVENT: Event = Event::AgentReassigned;
    const FROM: &'static [Status] = &[Status::Assigned];
    const TO: Status = Status::Assigned;
}

impl Rule<TicketMachine> for AgentReassigned {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(
            step.input.agent != step.entity.agent,
            "the agent is already assigned",
        )
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.agent = step.input.agent;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyAgent("reassigned")]
    }
}

struct WorkStarted;

impl Edge<TicketMachine> for WorkStarted {
    const EVENT: Event = Event::WorkStarted;
    const FROM: &'static [Status] = &[Status::Assigned];
    const TO: Status = Status::InProgress;
}

impl Rule<TicketMachine> for WorkStarted {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.agent.is_some(), "no agent assigned")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.diagnosis = step.input.diagnosis.or(ticket.diagnosis);
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("in_progress")]
    }
}

struct InfoRequested;

impl Edge<TicketMachine> for InfoRequested {
    const EVENT: Event = Event::InfoRequested;
    const FROM: &'static [Status] = &[Status::InProgress];
    const TO: Status = Status::Waiting;
}

impl Rule<TicketMachine> for InfoRequested {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.diagnosis.is_some(), "no active diagnosis")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.sla_paused = true;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::PauseSla]
    }
}

struct CustomerReplied;

impl Edge<TicketMachine> for CustomerReplied {
    const EVENT: Event = Event::CustomerReplied;
    const FROM: &'static [Status] = &[Status::Waiting];
    const TO: Status = Status::InProgress;
}

impl Rule<TicketMachine> for CustomerReplied {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.sla_paused, "the ticket is not waiting")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.sla_paused = false;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::ResumeSla]
    }
}

struct FixDeployed;

impl Edge<TicketMachine> for FixDeployed {
    const EVENT: Event = Event::FixDeployed;
    const FROM: &'static [Status] = &[Status::InProgress];
    const TO: Status = Status::Resolved;
}

impl Rule<TicketMachine> for FixDeployed {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.diagnosis.is_some(), "no diagnosis to resolve")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.resolution = ticket.diagnosis;
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("resolved")]
    }
}

struct FixRejected;

impl Edge<TicketMachine> for FixRejected {
    const EVENT: Event = Event::FixRejected;
    const FROM: &'static [Status] = &[Status::Resolved];
    const TO: Status = Status::Reopened;
}

impl Rule<TicketMachine> for FixRejected {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.reason.is_some(), "missing rejection reason")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.reopen_count += 1;
        ticket.resolution = None;
    }
}

struct NewDiagnosisStarted;

impl Edge<TicketMachine> for NewDiagnosisStarted {
    const EVENT: Event = Event::NewDiagnosisStarted;
    const FROM: &'static [Status] = &[Status::Reopened];
    const TO: Status = Status::InProgress;
}

impl Rule<TicketMachine> for NewDiagnosisStarted {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.entity.reopen_count > 0, "the ticket was not reopened")
    }

    fn apply(&self, step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.diagnosis = step.input.diagnosis.or(ticket.diagnosis);
    }
}

struct SolutionConfirmed;

impl Edge<TicketMachine> for SolutionConfirmed {
    const EVENT: Event = Event::SolutionConfirmed;
    const FROM: &'static [Status] = &[Status::Resolved];
    const TO: Status = Status::Closed;
}

impl Rule<TicketMachine> for SolutionConfirmed {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.confirmed, "the customer has not confirmed")?;
        expect(step.entity.resolution.is_some(), "no resolution recorded")?;
        expect(!step.from_any, "this transition cannot be overridden")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.closed_reason = Some("confirmed by the customer");
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::SendSurvey]
    }
}

struct InactivityTimeout {
    hours: u32,
}

impl Edge<TicketMachine> for InactivityTimeout {
    const EVENT: Event = Event::InactivityTimeout;
    const FROM: &'static [Status] = &[Status::Waiting];
    const TO: Status = Status::Closed;
}

impl Rule<TicketMachine> for InactivityTimeout {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(
            step.input.inactive_hours >= self.hours,
            "the inactivity period has not elapsed",
        )
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.closed_reason = Some("closed by inactivity");
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _ticket: &Ticket) -> Vec<TicketEffect> {
        vec![TicketEffect::NotifyRequester("closed_by_inactivity")]
    }
}

struct SupervisorOverride;

impl Edge<TicketMachine> for SupervisorOverride {
    const EVENT: Event = Event::SupervisorOverride;
    const FROM: &'static [Status] = &[];
    const TO: Status = Status::Closed;
}

impl Rule<TicketMachine> for SupervisorOverride {
    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        expect(step.input.actor == Actor::Supervisor, "supervisor required")
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.closed_reason = Some("overridden by supervisor");
    }
}

fn ticket_engine() -> Result<Engine<TicketMachine>, BuildErrors<TicketMachine>> {
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
        // From-any-only operation
        .edge(SupervisorOverride)
        // Global rules by scope
        .rule(
            Scope::always(),
            emit("publish_update", TicketEffect::PublishUpdate),
        )
        .rule(
            Scope::leaving(&[Status::Draft]),
            emit(
                "notify_submission",
                TicketEffect::NotifyRequester("ticket_submitted"),
            ),
        )
        .rule(
            Scope::entering(&[Status::Open]),
            emit("start_sla", TicketEffect::StartSla),
        )
        .rule(
            Scope::entering(&[Status::Reopened]),
            emit(
                "alert_recurrence",
                TicketEffect::NotifyRequester("recurrence"),
            ),
        )
        .rule(
            Scope::when(|step| step.from == Status::Assigned),
            emit("sync_assignment", TicketEffect::SyncAssignment),
        )
        .rule(
            Scope::on(&[Event::FixRejected]),
            emit(
                "notify_rejection",
                TicketEffect::NotifyRequester("fix_rejected"),
            ),
        )
        .rule(
            Scope::when(|step| step.from_any),
            emit("override_audit", TicketEffect::AuditOverride),
        )
        .initial(Status::Initial)
        .require_events([
            Event::NewDraft,
            Event::DirectTicketCreated,
            Event::TicketSubmitted,
            Event::AgentAssigned,
            Event::AgentReassigned,
            Event::WorkStarted,
            Event::InfoRequested,
            Event::CustomerReplied,
            Event::FixDeployed,
            Event::FixRejected,
            Event::NewDiagnosisStarted,
            Event::SolutionConfirmed,
            Event::InactivityTimeout,
            Event::SupervisorOverride,
        ])
        .require_states([
            Status::Initial,
            Status::Draft,
            Status::Open,
            Status::Assigned,
            Status::InProgress,
            Status::Waiting,
            Status::Resolved,
            Status::Reopened,
            Status::Closed,
        ])
        .build()
}

fn execute(
    engine: &Engine<TicketMachine>,
    ticket: &mut Ticket,
    event: Event,
    command: Command,
) -> Outcome<TicketMachine> {
    let outcome = engine
        .execute(ticket, event, &command)
        .expect("transition should succeed");
    // The caller keeps the outcome, so the entity is copied back into the ticket.
    *ticket = outcome.entity.clone();
    outcome
}

fn reject(
    engine: &Engine<TicketMachine>,
    ticket: &Ticket,
    event: Event,
    command: Command,
) -> Rejection<TicketMachine> {
    engine
        .execute(ticket, event, &command)
        .expect_err("transition should fail")
}

fn effects(outcome: &Outcome<TicketMachine>) -> Vec<TicketEffect> {
    outcome.effect_values().copied().collect()
}

fn open_ticket(engine: &Engine<TicketMachine>) -> Ticket {
    let mut ticket = Ticket::initial();
    execute(
        engine,
        &mut ticket,
        Event::DirectTicketCreated,
        Command {
            actor: Actor::Requester,
            folio: Some(4821),
            priority: Some(Priority::Normal),
            ..Command::default()
        },
    );
    ticket
}

fn assigned_ticket(engine: &Engine<TicketMachine>) -> Ticket {
    let mut ticket = open_ticket(engine);
    execute(
        engine,
        &mut ticket,
        Event::AgentAssigned,
        Command {
            actor: Actor::Agent,
            agent: Some("ana"),
            ..Command::default()
        },
    );
    ticket
}

fn in_progress_ticket(engine: &Engine<TicketMachine>) -> Ticket {
    let mut ticket = assigned_ticket(engine);
    execute(
        engine,
        &mut ticket,
        Event::WorkStarted,
        Command {
            actor: Actor::Agent,
            diagnosis: Some("corrupted service cache"),
            ..Command::default()
        },
    );
    ticket
}

fn resolved_ticket(engine: &Engine<TicketMachine>) -> Ticket {
    let mut ticket = in_progress_ticket(engine);
    execute(
        engine,
        &mut ticket,
        Event::FixDeployed,
        Command {
            actor: Actor::Agent,
            ..Command::default()
        },
    );
    ticket
}

fn waiting_ticket(engine: &Engine<TicketMachine>) -> Ticket {
    let mut ticket = in_progress_ticket(engine);
    execute(
        engine,
        &mut ticket,
        Event::InfoRequested,
        Command {
            actor: Actor::Agent,
            reason: Some("missing logs"),
            ..Command::default()
        },
    );
    ticket
}

#[test]
fn direct_creation_opens_the_ticket_and_starts_the_sla() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = Ticket::initial();

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::DirectTicketCreated,
        Command {
            actor: Actor::Requester,
            folio: Some(4821),
            priority: Some(Priority::High),
            ..Command::default()
        },
    );

    assert_eq!(outcome.transition.from, Status::Initial);
    assert_eq!(outcome.transition.to, Status::Open);
    assert_eq!(ticket.status, Status::Open);
    assert_eq!(ticket.folio, Some(4821));
    assert_eq!(ticket.priority, Priority::High);
    assert_eq!(
        outcome.transition.rules,
        vec!["DirectTicketCreated", "publish_update", "start_sla"]
    );
    assert_eq!(
        effects(&outcome),
        vec![
            TicketEffect::NotifyRequester("ticket_created"),
            TicketEffect::PublishUpdate,
            TicketEffect::StartSla,
        ]
    );
}

#[test]
fn draft_requires_a_description() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = Ticket::initial();

    let rejection = reject(&engine, &ticket, Event::NewDraft, Command::default());
    let errors = rejection.rules().expect("rule rejection");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].rule, "NewDraft");
    assert_eq!(ticket.status, Status::Initial);

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::NewDraft,
        Command {
            actor: Actor::Requester,
            description: Some("The shipping panel does not load"),
            ..Command::default()
        },
    );

    assert_eq!(ticket.status, Status::Draft);
    assert_eq!(ticket.description, Some("The shipping panel does not load"));
    assert_eq!(outcome.transition.rules, vec!["NewDraft", "publish_update"]);
    assert_eq!(
        effects(&outcome),
        vec![
            TicketEffect::NotifyRequester("draft_created"),
            TicketEffect::PublishUpdate,
        ]
    );
}

#[test]
fn submission_requires_a_folio_and_starts_the_sla() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = Ticket::initial();
    execute(
        &engine,
        &mut ticket,
        Event::NewDraft,
        Command {
            description: Some("The shipping panel does not load"),
            ..Command::default()
        },
    );

    let rejection = reject(&engine, &ticket, Event::TicketSubmitted, Command::default());
    assert_eq!(
        rejection.rules().expect("rule rejection")[0].rule,
        "TicketSubmitted"
    );

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::TicketSubmitted,
        Command {
            folio: Some(4821),
            ..Command::default()
        },
    );

    assert_eq!(ticket.status, Status::Open);
    assert_eq!(ticket.folio, Some(4821));
    assert_eq!(
        outcome.transition.rules,
        vec![
            "TicketSubmitted",
            "publish_update",
            "notify_submission",
            "start_sla",
        ]
    );
    assert_eq!(
        effects(&outcome),
        vec![
            TicketEffect::PublishUpdate,
            TicketEffect::NotifyRequester("ticket_submitted"),
            TicketEffect::StartSla,
        ]
    );
}

#[test]
fn assignment_reassignment_and_work_start() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = open_ticket(&engine);

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::AgentAssigned,
        Command {
            actor: Actor::Agent,
            agent: Some("ana"),
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::Assigned);
    assert_eq!(ticket.agent, Some("ana"));
    assert_eq!(
        effects(&outcome),
        vec![
            TicketEffect::NotifyAgent("ticket_assigned"),
            TicketEffect::PublishUpdate,
        ]
    );

    let rejection = reject(
        &engine,
        &ticket,
        Event::AgentReassigned,
        Command {
            actor: Actor::Agent,
            agent: Some("ana"),
            ..Command::default()
        },
    );
    assert_eq!(
        rejection.rules().expect("rule rejection")[0].rule,
        "AgentReassigned"
    );

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::AgentReassigned,
        Command {
            actor: Actor::Agent,
            agent: Some("luis"),
            ..Command::default()
        },
    );
    assert!(outcome.transition.is_self_loop());
    assert_eq!(ticket.agent, Some("luis"));
    assert_eq!(
        outcome.transition.rules,
        vec!["AgentReassigned", "publish_update", "sync_assignment"]
    );
    assert_eq!(
        effects(&outcome),
        vec![
            TicketEffect::NotifyAgent("reassigned"),
            TicketEffect::PublishUpdate,
            TicketEffect::SyncAssignment,
        ]
    );

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::WorkStarted,
        Command {
            actor: Actor::Agent,
            diagnosis: Some("corrupted service cache"),
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::InProgress);
    assert_eq!(ticket.diagnosis, Some("corrupted service cache"));
    assert_eq!(
        effects(&outcome),
        vec![
            TicketEffect::NotifyRequester("in_progress"),
            TicketEffect::PublishUpdate,
            TicketEffect::SyncAssignment,
        ]
    );
}

#[test]
fn waiting_pauses_the_sla_and_reply_resumes_it() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = in_progress_ticket(&engine);

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::InfoRequested,
        Command {
            actor: Actor::Agent,
            reason: Some("missing logs"),
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::Waiting);
    assert!(ticket.sla_paused);
    assert_eq!(
        effects(&outcome),
        vec![TicketEffect::PauseSla, TicketEffect::PublishUpdate]
    );

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::CustomerReplied,
        Command {
            actor: Actor::Requester,
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::InProgress);
    assert!(!ticket.sla_paused);
    assert_eq!(
        effects(&outcome),
        vec![TicketEffect::ResumeSla, TicketEffect::PublishUpdate,]
    );
}

#[test]
fn a_rejected_fix_reopens_the_ticket_and_starts_a_new_diagnosis() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = resolved_ticket(&engine);

    assert_eq!(ticket.status, Status::Resolved);
    assert_eq!(ticket.resolution, Some("corrupted service cache"));

    let rejection = reject(&engine, &ticket, Event::FixRejected, Command::default());
    assert_eq!(
        rejection.rules().expect("rule rejection")[0].rule,
        "FixRejected"
    );

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::FixRejected,
        Command {
            actor: Actor::Requester,
            reason: Some("the error persists"),
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::Reopened);
    assert_eq!(ticket.reopen_count, 1);
    assert_eq!(ticket.resolution, None);
    assert_eq!(
        effects(&outcome),
        vec![
            TicketEffect::PublishUpdate,
            TicketEffect::NotifyRequester("recurrence"),
            TicketEffect::NotifyRequester("fix_rejected"),
        ]
    );

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::NewDiagnosisStarted,
        Command {
            actor: Actor::Agent,
            diagnosis: Some("stale connection pool"),
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::InProgress);
    assert_eq!(ticket.diagnosis, Some("stale connection pool"));
    assert_eq!(
        outcome.transition.rules,
        vec!["NewDiagnosisStarted", "publish_update"]
    );
}

#[test]
fn solution_confirmation_requires_the_customer_and_rejects_override() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = resolved_ticket(&engine);

    let rejection = reject(
        &engine,
        &ticket,
        Event::SolutionConfirmed,
        Command::default(),
    );
    assert_eq!(
        rejection.rules().expect("rule rejection")[0].rule,
        "SolutionConfirmed"
    );

    let rejection = engine
        .execute_from_any(
            &ticket,
            Event::SolutionConfirmed,
            &Command {
                actor: Actor::Supervisor,
                confirmed: true,
                ..Command::default()
            },
        )
        .expect_err("the transition is not overridable");
    assert_eq!(
        rejection.rules().expect("rule rejection")[0].rule,
        "SolutionConfirmed"
    );
    assert_eq!(ticket.status, Status::Resolved);

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::SolutionConfirmed,
        Command {
            actor: Actor::Requester,
            confirmed: true,
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::Closed);
    assert_eq!(ticket.closed_reason, Some("confirmed by the customer"));
    assert_eq!(
        outcome.transition.rules,
        vec!["SolutionConfirmed", "publish_update"]
    );
    assert_eq!(
        effects(&outcome),
        vec![TicketEffect::SendSurvey, TicketEffect::PublishUpdate]
    );
}

#[test]
fn inactivity_timeout_closes_a_waiting_ticket() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = waiting_ticket(&engine);

    let rejection = reject(
        &engine,
        &ticket,
        Event::InactivityTimeout,
        Command {
            inactive_hours: 71,
            ..Command::default()
        },
    );
    assert_eq!(
        rejection.rules().expect("rule rejection")[0].rule,
        "InactivityTimeout"
    );
    assert_eq!(ticket.status, Status::Waiting);

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::InactivityTimeout,
        Command {
            inactive_hours: 72,
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::Closed);
    assert_eq!(ticket.closed_reason, Some("closed by inactivity"));
    assert_eq!(
        effects(&outcome),
        vec![
            TicketEffect::NotifyRequester("closed_by_inactivity"),
            TicketEffect::PublishUpdate,
        ]
    );
}

#[test]
fn supervisor_override_closes_from_any_state() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = open_ticket(&engine);

    let rejection = reject(
        &engine,
        &ticket,
        Event::SupervisorOverride,
        Command::default(),
    );
    assert!(matches!(rejection, Rejection::NotAllowed { .. }));

    let rejection = engine
        .execute_from_any(&ticket, Event::SupervisorOverride, &Command::default())
        .expect_err("only a supervisor can override");
    assert_eq!(
        rejection.rules().expect("rule rejection")[0].rule,
        "SupervisorOverride"
    );

    let outcome = engine
        .execute_from_any(
            &ticket,
            Event::SupervisorOverride,
            &Command {
                actor: Actor::Supervisor,
                ..Command::default()
            },
        )
        .expect("supervisor override is valid");
    ticket = outcome.entity.clone();

    assert_eq!(ticket.status, Status::Closed);
    assert_eq!(ticket.closed_reason, Some("overridden by supervisor"));
    assert!(outcome.transition.from_any);
    assert_eq!(
        effects(&outcome),
        vec![TicketEffect::PublishUpdate, TicketEffect::AuditOverride]
    );
}

#[test]
fn available_and_available_detailed_explain_the_next_actions() {
    let engine = ticket_engine().expect("valid ticket machine");
    let ticket = open_ticket(&engine);

    assert_eq!(engine.available(&ticket), vec![Event::AgentAssigned]);

    let detailed = engine.available_detailed(&ticket, &Command::default());
    assert_eq!(detailed.len(), 1);
    let (event, verdict) = &detailed[0];
    assert_eq!(*event, Event::AgentAssigned);
    let rejection = verdict.as_ref().expect_err("agent is missing");
    assert_eq!(
        rejection.rules().expect("rule rejection")[0].rule,
        "AgentAssigned"
    );

    let assigned = assigned_ticket(&engine);
    assert_eq!(
        engine.available(&assigned),
        vec![Event::AgentReassigned, Event::WorkStarted]
    );
}

#[test]
fn happy_path_reaches_closed_with_all_effects() {
    let engine = ticket_engine().expect("valid ticket machine");
    let mut ticket = in_progress_ticket(&engine);

    execute(
        &engine,
        &mut ticket,
        Event::FixDeployed,
        Command {
            actor: Actor::Agent,
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::Resolved);

    let outcome = execute(
        &engine,
        &mut ticket,
        Event::SolutionConfirmed,
        Command {
            actor: Actor::Requester,
            confirmed: true,
            ..Command::default()
        },
    );
    assert_eq!(ticket.status, Status::Closed);
    assert_eq!(
        effects(&outcome),
        vec![TicketEffect::SendSurvey, TicketEffect::PublishUpdate]
    );
}

#[test]
fn mermaid_marks_the_root_and_the_override_note() {
    let diagram = ticket_engine().expect("valid ticket machine").to_mermaid();

    assert!(diagram.contains("    [*] --> Initial"), "{diagram}");
    assert!(
        diagram.contains("    Resolved --> Closed : SolutionConfirmed"),
        "{diagram}"
    );
    assert!(
        diagram.contains("    note right of Closed : SupervisorOverride"),
        "{diagram}"
    );
    assert!(!diagram.contains("forced only"), "{diagram}");
}

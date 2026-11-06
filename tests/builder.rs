//! Builder validation: topology errors, roots, reachability, and edges without origin.

use fsm_forge::{BuildError, BuildErrors, Edge, Engine, Machine, Rejection, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ShipmentState {
    Initial,
    Draft,
    Confirmed,
    Ready,
    Delivered,
    Cancelled,
    Orphan,
    OrphanNext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ShipmentEvent {
    Create,
    Confirm,
    Dispatch,
    Deliver,
    Cancel,
    AdminCancel,
    OrphanStep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Shipment {
    state: ShipmentState,
}

/// Shipment machine: builder validation, roots, conflicts, missing pieces, and
/// reachability.
///
/// ```mermaid
/// stateDiagram-v2
///     state "Initial" as Initial
///     state "Draft" as Draft
///     state "Confirmed" as Confirmed
///     state "Ready" as Ready
///     state "Delivered" as Delivered
///     state "Cancelled" as Cancelled
///     [*] --> Initial
///     [*] --> Draft
///     Initial --> Draft : Create
///     Draft --> Confirmed : Confirm
///     Confirmed --> Ready : Dispatch
///     Ready --> Delivered : Deliver
///     Draft --> Cancelled : Cancel
///     Confirmed --> Cancelled : Cancel
///     note right of Cancelled : AdminCancel
/// ```
struct ShipmentMachine;

impl Machine for ShipmentMachine {
    type State = ShipmentState;
    type Event = ShipmentEvent;
    type Entity = Shipment;
    type Input = ();
    type Effect = ();
    type Error = &'static str;

    fn state(shipment: &Shipment) -> ShipmentState {
        shipment.state
    }

    fn set_state(shipment: &mut Shipment, state: ShipmentState) {
        shipment.state = state;
    }
}

struct CreateShipment;

impl Edge<ShipmentMachine> for CreateShipment {
    const EVENT: ShipmentEvent = ShipmentEvent::Create;
    const FROM: &'static [ShipmentState] = &[ShipmentState::Initial];
    const TO: ShipmentState = ShipmentState::Draft;
}

impl Rule<ShipmentMachine> for CreateShipment {}

struct ConfirmShipment;

impl Edge<ShipmentMachine> for ConfirmShipment {
    const EVENT: ShipmentEvent = ShipmentEvent::Confirm;
    const FROM: &'static [ShipmentState] = &[ShipmentState::Draft];
    const TO: ShipmentState = ShipmentState::Confirmed;
}

impl Rule<ShipmentMachine> for ConfirmShipment {}

struct DispatchShipment;

impl Edge<ShipmentMachine> for DispatchShipment {
    const EVENT: ShipmentEvent = ShipmentEvent::Dispatch;
    const FROM: &'static [ShipmentState] = &[ShipmentState::Confirmed];
    const TO: ShipmentState = ShipmentState::Ready;
}

impl Rule<ShipmentMachine> for DispatchShipment {}

struct DeliverShipment;

impl Edge<ShipmentMachine> for DeliverShipment {
    const EVENT: ShipmentEvent = ShipmentEvent::Deliver;
    const FROM: &'static [ShipmentState] = &[ShipmentState::Ready];
    const TO: ShipmentState = ShipmentState::Delivered;
}

impl Rule<ShipmentMachine> for DeliverShipment {}

struct CancelDraft;

impl Edge<ShipmentMachine> for CancelDraft {
    const EVENT: ShipmentEvent = ShipmentEvent::Cancel;
    const FROM: &'static [ShipmentState] = &[ShipmentState::Draft];
    const TO: ShipmentState = ShipmentState::Cancelled;
}

impl Rule<ShipmentMachine> for CancelDraft {}

struct CancelConfirmed;

impl Edge<ShipmentMachine> for CancelConfirmed {
    const EVENT: ShipmentEvent = ShipmentEvent::Cancel;
    const FROM: &'static [ShipmentState] = &[ShipmentState::Confirmed];
    const TO: ShipmentState = ShipmentState::Cancelled;
}

impl Rule<ShipmentMachine> for CancelConfirmed {}

struct AdminCancel;

impl Edge<ShipmentMachine> for AdminCancel {
    const EVENT: ShipmentEvent = ShipmentEvent::AdminCancel;
    const FROM: &'static [ShipmentState] = &[];
    const TO: ShipmentState = ShipmentState::Cancelled;
}

impl Rule<ShipmentMachine> for AdminCancel {}

/// Same event and origin as [`ConfirmShipment`], different target: used to prove
/// that `build()` reports the conflict.
struct ConfirmDraftToCancelled;

impl Edge<ShipmentMachine> for ConfirmDraftToCancelled {
    const EVENT: ShipmentEvent = ShipmentEvent::Confirm;
    const FROM: &'static [ShipmentState] = &[ShipmentState::Draft];
    const TO: ShipmentState = ShipmentState::Cancelled;
}

impl Rule<ShipmentMachine> for ConfirmDraftToCancelled {}

/// Second edge without origin for `AdminCancel`: used to prove the ambiguity.
struct AdminCancelToDelivered;

impl Edge<ShipmentMachine> for AdminCancelToDelivered {
    const EVENT: ShipmentEvent = ShipmentEvent::AdminCancel;
    const FROM: &'static [ShipmentState] = &[];
    const TO: ShipmentState = ShipmentState::Delivered;
}

impl Rule<ShipmentMachine> for AdminCancelToDelivered {}

struct OrphanStep;

impl Edge<ShipmentMachine> for OrphanStep {
    const EVENT: ShipmentEvent = ShipmentEvent::OrphanStep;
    const FROM: &'static [ShipmentState] = &[ShipmentState::Orphan];
    const TO: ShipmentState = ShipmentState::OrphanNext;
}

impl Rule<ShipmentMachine> for OrphanStep {}

fn valid_engine() -> Result<Engine<ShipmentMachine>, BuildErrors<ShipmentMachine>> {
    Engine::<ShipmentMachine>::builder()
        .edge(CreateShipment)
        .edge(ConfirmShipment)
        .edge(DispatchShipment)
        .edge(DeliverShipment)
        .edge(CancelDraft)
        .edge(CancelConfirmed)
        .edge(AdminCancel)
        .roots([ShipmentState::Initial, ShipmentState::Draft])
        .require_events([
            ShipmentEvent::Create,
            ShipmentEvent::Confirm,
            ShipmentEvent::Dispatch,
            ShipmentEvent::Deliver,
            ShipmentEvent::Cancel,
            ShipmentEvent::AdminCancel,
        ])
        .require_states([
            ShipmentState::Initial,
            ShipmentState::Draft,
            ShipmentState::Confirmed,
            ShipmentState::Ready,
            ShipmentState::Delivered,
            ShipmentState::Cancelled,
        ])
        .build()
}

fn expect_build_error(
    result: Result<Engine<ShipmentMachine>, BuildErrors<ShipmentMachine>>,
) -> BuildErrors<ShipmentMachine> {
    match result {
        Err(errors) => errors,
        Ok(_) => panic!("expected the build to fail"),
    }
}

#[test]
fn valid_build_exposes_roots_and_targets() {
    let engine = valid_engine().expect("valid shipment machine");

    assert_eq!(engine.initial(), Some(ShipmentState::Initial));
    assert_eq!(
        engine.roots(),
        &[ShipmentState::Initial, ShipmentState::Draft]
    );
    assert_eq!(
        engine.target(ShipmentEvent::Confirm, ShipmentState::Draft),
        Some(ShipmentState::Confirmed)
    );
    assert_eq!(
        engine.target(ShipmentEvent::AdminCancel, ShipmentState::Initial),
        None
    );
}

#[test]
fn edge_without_origin_is_valid_and_requires_from_any() {
    let engine = valid_engine().expect("valid shipment machine");
    let shipment = Shipment {
        state: ShipmentState::Ready,
    };

    let rejection = engine
        .execute(&shipment, ShipmentEvent::AdminCancel, &())
        .expect_err("edges without origin are not available normally");
    assert!(matches!(rejection, Rejection::NotAllowed { .. }));

    let outcome = engine
        .execute_from_any(&shipment, ShipmentEvent::AdminCancel, &())
        .expect("from_any can use the edge without origin");
    assert_eq!(outcome.transition.to, ShipmentState::Cancelled);
    assert!(outcome.transition.from_any);
}

#[test]
fn missing_event_is_reported() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(CreateShipment)
        .require_events([ShipmentEvent::Create, ShipmentEvent::Confirm])
        .initial(ShipmentState::Initial)
        .build();

    let errors = expect_build_error(result);
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, BuildError::MissingEvent(ShipmentEvent::Confirm)))
    );
}

#[test]
fn missing_state_is_reported() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(CreateShipment)
        .require_states([ShipmentState::Draft, ShipmentState::Orphan])
        .initial(ShipmentState::Initial)
        .build();

    let errors = expect_build_error(result);
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, BuildError::MissingState(ShipmentState::Orphan)))
    );
}

#[test]
fn a_root_counts_as_a_mentioned_state() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(CreateShipment)
        .require_states([ShipmentState::Initial, ShipmentState::Draft])
        .initial(ShipmentState::Initial)
        .build();

    assert!(result.is_ok());
}

#[test]
fn conflicting_event_and_origin_is_reported() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(ConfirmShipment)
        .edge(ConfirmDraftToCancelled)
        .initial(ShipmentState::Draft)
        .build();

    let errors = expect_build_error(result);
    assert!(errors.iter().any(|error| matches!(
        error,
        BuildError::Conflict {
            event: ShipmentEvent::Confirm,
            from: ShipmentState::Draft,
        }
    )));
}

#[test]
fn several_edges_without_origin_are_ambiguous() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(AdminCancel)
        .edge(AdminCancelToDelivered)
        .initial(ShipmentState::Initial)
        .build();

    let errors = expect_build_error(result);
    assert!(errors.iter().any(|error| matches!(
        error,
        BuildError::AmbiguousOriginless {
            event: ShipmentEvent::AdminCancel,
        }
    )));
}

#[test]
fn an_ambiguous_event_is_reported_once_however_many_edges_repeat_it() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(AdminCancel)
        .edge(AdminCancelToDelivered)
        .edge(AdminCancel)
        .initial(ShipmentState::Initial)
        .build();

    let errors = expect_build_error(result);
    let ambiguous = errors
        .iter()
        .filter(|error| matches!(error, BuildError::AmbiguousOriginless { .. }))
        .count();
    assert_eq!(ambiguous, 1, "{errors}");
}

#[test]
fn unreachable_states_are_reported_but_reachable_ones_are_not() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(CreateShipment)
        .edge(OrphanStep)
        .initial(ShipmentState::Initial)
        .build();

    let errors = expect_build_error(result);
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, BuildError::UnreachableState(ShipmentState::Orphan)))
    );
    assert!(errors.iter().any(|error| matches!(
        error,
        BuildError::UnreachableState(ShipmentState::OrphanNext)
    )));
    assert!(
        !errors
            .iter()
            .any(|error| matches!(error, BuildError::UnreachableState(ShipmentState::Draft)))
    );
}

#[test]
fn build_reports_every_independent_error() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(ConfirmShipment)
        .edge(ConfirmDraftToCancelled)
        .require_events([ShipmentEvent::Create])
        .require_states([ShipmentState::Orphan])
        .initial(ShipmentState::Draft)
        .build();

    let errors = expect_build_error(result);
    assert_eq!(errors.len(), 3);
    assert!(errors.iter().any(|error| matches!(
        error,
        BuildError::Conflict {
            event: ShipmentEvent::Confirm,
            from: ShipmentState::Draft,
        }
    )));
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, BuildError::MissingEvent(ShipmentEvent::Create)))
    );
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, BuildError::MissingState(ShipmentState::Orphan)))
    );

    let message = errors.to_string();
    assert!(message.contains("has two edges"), "{message}");
    assert!(message.contains("has no edge"), "{message}");
}

#[test]
fn edge_without_origin_counts_for_reachability() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(AdminCancel)
        .require_states([ShipmentState::Cancelled])
        .initial(ShipmentState::Initial)
        .build();

    assert!(result.is_ok());
}

#[test]
fn without_roots_reachability_is_not_checked() {
    let result = Engine::<ShipmentMachine>::builder()
        .edge(OrphanStep)
        .build();

    let engine = result.expect("no roots means no reachability check");
    assert_eq!(engine.initial(), None);
    assert!(engine.roots().is_empty());
}

#[test]
fn isolated_root_is_valid() {
    let engine = Engine::<ShipmentMachine>::builder()
        .initial(ShipmentState::Initial)
        .build()
        .expect("a root without topology is valid");

    assert_eq!(engine.initial(), Some(ShipmentState::Initial));
    assert_eq!(engine.roots(), &[ShipmentState::Initial]);
}

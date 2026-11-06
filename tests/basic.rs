//! Simple topologies: no rules, self-loops, several origins, and from-any mode.

use fsm_forge::{Edge, Engine, Machine, Rejection, Rule};

// ─────────────────────────────────────────────────────────────────────────────
// Door: basic edges, a self-loop, several origins, and an edge without origin
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DoorState {
    Closed,
    Open,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DoorEvent {
    OpenDoor,
    CloseDoor,
    HoldOpen,
    LockDoor,
    UnlockDoor,
    ForceOpen,
    RingBell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Door {
    state: DoorState,
}

/// Door machine: simple edges, a self-loop, several origins, and an edge
/// without origin.
///
/// ```mermaid
/// stateDiagram-v2
///     state "Closed" as Closed
///     state "Open" as Open
///     state "Locked" as Locked
///     [*] --> Closed
///     Closed --> Open : OpenDoor
///     Open --> Closed : CloseDoor
///     Open --> Open : HoldOpen
///     Closed --> Locked : LockDoor
///     Open --> Locked : LockDoor
///     Locked --> Closed : UnlockDoor
///     note right of Open : ForceOpen
/// ```
struct DoorMachine;

impl Machine for DoorMachine {
    type State = DoorState;
    type Event = DoorEvent;
    type Entity = Door;
    type Input = ();
    type Effect = ();
    type Error = &'static str;

    fn state(door: &Door) -> DoorState {
        door.state
    }

    fn set_state(door: &mut Door, state: DoorState) {
        door.state = state;
    }
}

struct OpenDoor;

impl Edge<DoorMachine> for OpenDoor {
    const EVENT: DoorEvent = DoorEvent::OpenDoor;
    const FROM: &'static [DoorState] = &[DoorState::Closed];
    const TO: DoorState = DoorState::Open;
}

impl Rule<DoorMachine> for OpenDoor {}

struct CloseDoor;

impl Edge<DoorMachine> for CloseDoor {
    const EVENT: DoorEvent = DoorEvent::CloseDoor;
    const FROM: &'static [DoorState] = &[DoorState::Open];
    const TO: DoorState = DoorState::Closed;
}

impl Rule<DoorMachine> for CloseDoor {}

struct HoldOpen;

impl Edge<DoorMachine> for HoldOpen {
    const EVENT: DoorEvent = DoorEvent::HoldOpen;
    const FROM: &'static [DoorState] = &[DoorState::Open];
    const TO: DoorState = DoorState::Open;
}

impl Rule<DoorMachine> for HoldOpen {}

struct LockDoorFromClosed;

impl Edge<DoorMachine> for LockDoorFromClosed {
    const EVENT: DoorEvent = DoorEvent::LockDoor;
    const FROM: &'static [DoorState] = &[DoorState::Closed];
    const TO: DoorState = DoorState::Locked;
}

impl Rule<DoorMachine> for LockDoorFromClosed {}

struct LockDoorFromOpen;

impl Edge<DoorMachine> for LockDoorFromOpen {
    const EVENT: DoorEvent = DoorEvent::LockDoor;
    const FROM: &'static [DoorState] = &[DoorState::Open];
    const TO: DoorState = DoorState::Locked;
}

impl Rule<DoorMachine> for LockDoorFromOpen {}

struct UnlockDoor;

impl Edge<DoorMachine> for UnlockDoor {
    const EVENT: DoorEvent = DoorEvent::UnlockDoor;
    const FROM: &'static [DoorState] = &[DoorState::Locked];
    const TO: DoorState = DoorState::Closed;
}

impl Rule<DoorMachine> for UnlockDoor {}

struct ForceOpen;

impl Edge<DoorMachine> for ForceOpen {
    const EVENT: DoorEvent = DoorEvent::ForceOpen;
    const FROM: &'static [DoorState] = &[];
    const TO: DoorState = DoorState::Open;
}

impl Rule<DoorMachine> for ForceOpen {}

fn door_engine() -> Engine<DoorMachine> {
    Engine::<DoorMachine>::builder()
        .edge(OpenDoor)
        .edge(CloseDoor)
        .edge(HoldOpen)
        .edge(LockDoorFromClosed)
        .edge(LockDoorFromOpen)
        .edge(UnlockDoor)
        .edge(ForceOpen)
        .initial(DoorState::Closed)
        .build()
        .expect("valid door machine")
}

#[test]
fn normal_transition_returns_a_new_entity() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Closed,
    };

    let outcome = engine
        .execute(&door, DoorEvent::OpenDoor, &())
        .expect("transition is valid");

    assert_eq!(outcome.transition.event, DoorEvent::OpenDoor);
    assert_eq!(outcome.transition.from, DoorState::Closed);
    assert_eq!(outcome.transition.to, DoorState::Open);
    assert_eq!(outcome.entity.state, DoorState::Open);
    assert!(!outcome.transition.from_any);
    assert_eq!(outcome.transition.rules, vec!["OpenDoor"]);
    assert!(outcome.effects.is_empty());
}

#[test]
fn original_entity_is_untouched() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Closed,
    };

    let outcome = engine
        .execute(&door, DoorEvent::OpenDoor, &())
        .expect("transition is valid");

    assert_eq!(door.state, DoorState::Closed);
    assert_eq!(outcome.entity.state, DoorState::Open);
}

#[test]
fn transition_from_wrong_state_is_not_allowed() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Locked,
    };

    let rejection = engine
        .execute(&door, DoorEvent::OpenDoor, &())
        .expect_err("locked door cannot open normally");

    assert!(matches!(
        rejection,
        Rejection::NotAllowed {
            event: DoorEvent::OpenDoor,
            from: DoorState::Locked,
        }
    ));
}

#[test]
fn self_loop_is_allowed_and_reported() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Open,
    };

    let outcome = engine
        .execute(&door, DoorEvent::HoldOpen, &())
        .expect("self-loop is valid");

    assert_eq!(outcome.transition.from, DoorState::Open);
    assert_eq!(outcome.transition.to, DoorState::Open);
    assert!(outcome.transition.is_self_loop());
}

#[test]
fn one_event_can_have_several_origins() {
    let engine = door_engine();

    for state in [DoorState::Closed, DoorState::Open] {
        let door = Door { state };
        let outcome = engine
            .execute(&door, DoorEvent::LockDoor, &())
            .expect("both origins are valid");
        assert_eq!(outcome.entity.state, DoorState::Locked);
    }
}

#[test]
fn edge_without_origin_requires_from_any() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Locked,
    };

    let rejection = engine
        .execute(&door, DoorEvent::ForceOpen, &())
        .expect_err("edges without origin are not available normally");
    assert!(matches!(
        rejection,
        Rejection::NotAllowed {
            event: DoorEvent::ForceOpen,
            from: DoorState::Locked,
        }
    ));

    let outcome = engine
        .execute_from_any(&door, DoorEvent::ForceOpen, &())
        .expect("from_any can use the edge without origin");
    assert_eq!(outcome.transition.from, DoorState::Locked);
    assert_eq!(outcome.transition.to, DoorState::Open);
    assert!(outcome.transition.from_any);
}

#[test]
fn from_any_can_use_a_unique_edge_from_another_state() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Locked,
    };

    let outcome = engine
        .execute_from_any(&door, DoorEvent::OpenDoor, &())
        .expect("OpenDoor has a single edge");
    assert_eq!(outcome.transition.from, DoorState::Locked);
    assert_eq!(outcome.transition.to, DoorState::Open);
    assert!(outcome.transition.from_any);
}

#[test]
fn from_any_rejects_an_ambiguous_event() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Locked,
    };

    let rejection = engine
        .execute_from_any(&door, DoorEvent::LockDoor, &())
        .expect_err("LockDoor has two edges and none starts at Locked");
    assert!(matches!(
        rejection,
        Rejection::Ambiguous(DoorEvent::LockDoor)
    ));
}

#[test]
fn unknown_event_is_rejected() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Closed,
    };

    let rejection = engine
        .execute(&door, DoorEvent::RingBell, &())
        .expect_err("RingBell has no edge");
    assert!(matches!(
        rejection,
        Rejection::UnknownEvent(DoorEvent::RingBell)
    ));
}

#[test]
fn diagnose_available_and_target_agree_with_the_topology() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Closed,
    };

    let diagnosis = engine
        .diagnose(&door, DoorEvent::OpenDoor, &())
        .expect("valid diagnosis");
    assert_eq!(diagnosis.from, DoorState::Closed);
    assert_eq!(diagnosis.to, DoorState::Open);
    assert!(!diagnosis.from_any);

    assert_eq!(
        engine.available(&door),
        vec![DoorEvent::OpenDoor, DoorEvent::LockDoor]
    );

    let detailed = engine.available_detailed(&door, &());
    assert_eq!(detailed.len(), 2);
    assert!(detailed.iter().all(|(_, verdict)| verdict.is_ok()));

    assert_eq!(
        engine.target(DoorEvent::LockDoor, DoorState::Open),
        Some(DoorState::Locked)
    );
    assert_eq!(engine.target(DoorEvent::ForceOpen, DoorState::Closed), None);
}

#[test]
fn execute_returns_the_updated_entity() {
    let engine = door_engine();
    let door = Door {
        state: DoorState::Closed,
    };

    let outcome = engine
        .execute(&door, DoorEvent::OpenDoor, &())
        .expect("transition is valid");

    assert_eq!(outcome.entity.state, DoorState::Open);
    assert_eq!(door.state, DoorState::Closed);
}

#[test]
fn mermaid_shows_the_root_and_the_originless_note() {
    let diagram = door_engine().to_mermaid();

    assert!(diagram.contains("    [*] --> Closed"), "{diagram}");
    assert!(
        diagram.contains("    Closed --> Open : OpenDoor"),
        "{diagram}"
    );
    assert!(
        diagram.contains("    note right of Open : ForceOpen"),
        "{diagram}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Traffic light: a cycle, a self-loop, and an originless reset
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LightState {
    Red,
    Green,
    Yellow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LightEvent {
    Next,
    Emergency,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Light {
    state: LightState,
}

/// Traffic light machine: a cycle, an emergency self-loop, and an edge without
/// origin.
///
/// ```mermaid
/// stateDiagram-v2
///     state "Red" as Red
///     state "Green" as Green
///     state "Yellow" as Yellow
///     [*] --> Red
///     Red --> Green : Next
///     Green --> Yellow : Next
///     Yellow --> Red : Next
///     Red --> Red : Emergency
///     Green --> Red : Emergency
///     Yellow --> Red : Emergency
///     note right of Red : Reset
/// ```
struct LightMachine;

impl Machine for LightMachine {
    type State = LightState;
    type Event = LightEvent;
    type Entity = Light;
    type Input = ();
    type Effect = ();
    type Error = &'static str;

    fn state(light: &Light) -> LightState {
        light.state
    }

    fn set_state(light: &mut Light, state: LightState) {
        light.state = state;
    }
}

struct NextFromRed;

impl Edge<LightMachine> for NextFromRed {
    const EVENT: LightEvent = LightEvent::Next;
    const FROM: &'static [LightState] = &[LightState::Red];
    const TO: LightState = LightState::Green;
}

impl Rule<LightMachine> for NextFromRed {}

struct NextFromGreen;

impl Edge<LightMachine> for NextFromGreen {
    const EVENT: LightEvent = LightEvent::Next;
    const FROM: &'static [LightState] = &[LightState::Green];
    const TO: LightState = LightState::Yellow;
}

impl Rule<LightMachine> for NextFromGreen {}

struct NextFromYellow;

impl Edge<LightMachine> for NextFromYellow {
    const EVENT: LightEvent = LightEvent::Next;
    const FROM: &'static [LightState] = &[LightState::Yellow];
    const TO: LightState = LightState::Red;
}

impl Rule<LightMachine> for NextFromYellow {}

struct Emergency;

impl Edge<LightMachine> for Emergency {
    const EVENT: LightEvent = LightEvent::Emergency;
    const FROM: &'static [LightState] = &[LightState::Red, LightState::Green, LightState::Yellow];
    const TO: LightState = LightState::Red;
}

impl Rule<LightMachine> for Emergency {}

struct Reset;

impl Edge<LightMachine> for Reset {
    const EVENT: LightEvent = LightEvent::Reset;
    const FROM: &'static [LightState] = &[];
    const TO: LightState = LightState::Red;
}

impl Rule<LightMachine> for Reset {}

fn light_engine() -> Engine<LightMachine> {
    Engine::<LightMachine>::builder()
        .edge(NextFromRed)
        .edge(NextFromGreen)
        .edge(NextFromYellow)
        .edge(Emergency)
        .edge(Reset)
        .initial(LightState::Red)
        .build()
        .expect("valid traffic light machine")
}

#[test]
fn traffic_light_cycles_through_every_state() {
    let engine = light_engine();
    let mut light = Light {
        state: LightState::Red,
    };

    for expected in [LightState::Green, LightState::Yellow, LightState::Red] {
        let outcome = engine
            .execute(&light, LightEvent::Next, &())
            .expect("Next is valid in every state");
        light = outcome.entity;
        assert_eq!(light.state, expected);
    }
}

#[test]
fn emergency_from_red_is_a_self_loop() {
    let engine = light_engine();
    let light = Light {
        state: LightState::Red,
    };

    let outcome = engine
        .execute(&light, LightEvent::Emergency, &())
        .expect("emergency is valid from red");
    assert_eq!(outcome.transition.from, LightState::Red);
    assert_eq!(outcome.transition.to, LightState::Red);
    assert!(outcome.transition.is_self_loop());
}

#[test]
fn reset_is_only_reachable_through_from_any() {
    let engine = light_engine();
    let light = Light {
        state: LightState::Green,
    };

    assert!(engine.execute(&light, LightEvent::Reset, &()).is_err());

    let outcome = engine
        .execute_from_any(&light, LightEvent::Reset, &())
        .expect("from_any can reset the light");
    assert_eq!(outcome.transition.to, LightState::Red);
    assert!(outcome.transition.from_any);
}

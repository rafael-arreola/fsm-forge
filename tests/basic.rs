//! Simple topologies: no rules, self-loops, several origins, and from-any mode.

use std::cell::Cell;
use std::rc::Rc;

use fsm_forge::{Edge, Engine, Machine, Rejection};

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

struct CloseDoor;

impl Edge<DoorMachine> for CloseDoor {
    const EVENT: DoorEvent = DoorEvent::CloseDoor;
    const FROM: &'static [DoorState] = &[DoorState::Open];
    const TO: DoorState = DoorState::Closed;
}

struct HoldOpen;

impl Edge<DoorMachine> for HoldOpen {
    const EVENT: DoorEvent = DoorEvent::HoldOpen;
    const FROM: &'static [DoorState] = &[DoorState::Open];
    const TO: DoorState = DoorState::Open;
}

struct LockDoorFromClosed;

impl Edge<DoorMachine> for LockDoorFromClosed {
    const EVENT: DoorEvent = DoorEvent::LockDoor;
    const FROM: &'static [DoorState] = &[DoorState::Closed];
    const TO: DoorState = DoorState::Locked;
}

struct LockDoorFromOpen;

impl Edge<DoorMachine> for LockDoorFromOpen {
    const EVENT: DoorEvent = DoorEvent::LockDoor;
    const FROM: &'static [DoorState] = &[DoorState::Open];
    const TO: DoorState = DoorState::Locked;
}

struct UnlockDoor;

impl Edge<DoorMachine> for UnlockDoor {
    const EVENT: DoorEvent = DoorEvent::UnlockDoor;
    const FROM: &'static [DoorState] = &[DoorState::Locked];
    const TO: DoorState = DoorState::Closed;
}

struct ForceOpen;

impl Edge<DoorMachine> for ForceOpen {
    const EVENT: DoorEvent = DoorEvent::ForceOpen;
    const FROM: &'static [DoorState] = &[];
    const TO: DoorState = DoorState::Open;
}

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
        &[DoorEvent::OpenDoor, DoorEvent::LockDoor]
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

struct NextFromGreen;

impl Edge<LightMachine> for NextFromGreen {
    const EVENT: LightEvent = LightEvent::Next;
    const FROM: &'static [LightState] = &[LightState::Green];
    const TO: LightState = LightState::Yellow;
}

struct NextFromYellow;

impl Edge<LightMachine> for NextFromYellow {
    const EVENT: LightEvent = LightEvent::Next;
    const FROM: &'static [LightState] = &[LightState::Yellow];
    const TO: LightState = LightState::Red;
}

struct Emergency;

impl Edge<LightMachine> for Emergency {
    const EVENT: LightEvent = LightEvent::Emergency;
    const FROM: &'static [LightState] = &[LightState::Red, LightState::Green, LightState::Yellow];
    const TO: LightState = LightState::Red;
}

struct Reset;

impl Edge<LightMachine> for Reset {
    const EVENT: LightEvent = LightEvent::Reset;
    const FROM: &'static [LightState] = &[];
    const TO: LightState = LightState::Red;
}

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

#[test]
fn engine_is_send_and_sync_even_when_entity_input_and_effect_are_not() {
    struct LocalMachine;

    impl Machine for LocalMachine {
        type State = u8;
        type Event = u8;
        type Entity = Rc<u8>;
        type Input = Rc<Cell<u8>>;
        type Effect = Rc<Cell<u8>>;
        type Error = &'static str;

        fn state(entity: &Self::Entity) -> u8 {
            **entity
        }

        fn set_state(entity: &mut Self::Entity, state: u8) {
            *entity = Rc::new(state);
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Engine<LocalMachine>>();
}

#[test]
fn custom_names_and_hostile_mermaid_labels_are_formatted_safely() {
    struct CustomMachine;

    impl Machine for CustomMachine {
        type State = u8;
        type Event = u8;
        type Entity = u8;
        type Input = ();
        type Effect = ();
        type Error = &'static str;

        fn state(entity: &u8) -> u8 {
            *entity
        }

        fn set_state(entity: &mut u8, state: u8) {
            *entity = state;
        }

        fn state_name(state: u8) -> String {
            match state {
                0 => String::new(),
                _ => "Active: 100%%".to_string(),
            }
        }

        fn event_name(event: u8) -> String {
            match event {
                1 => "Retry { max: 3 }; %%now".to_string(),
                _ => String::new(),
            }
        }
    }

    struct Go;

    impl Edge<CustomMachine> for Go {
        const EVENT: u8 = 1;
        const FROM: &'static [u8] = &[0];
        const TO: u8 = 1;
    }

    struct Force;

    impl Edge<CustomMachine> for Force {
        const EVENT: u8 = 2;
        const FROM: &'static [u8] = &[];
        const TO: u8 = 1;
    }

    let engine = Engine::<CustomMachine>::builder()
        .edge(Go)
        .edge(Force)
        .initial(0)
        .build()
        .expect("valid machine");

    let err = engine.execute(&1, 1, &()).expect_err("not allowed from 1");
    assert_eq!(
        err.to_string(),
        "Retry { max: 3 }; %%now is not allowed from Active: 100%%"
    );

    let diagram = engine.to_mermaid();
    assert!(diagram.contains("state \"0\" as 0"), "{diagram}");
    assert!(
        diagram.contains("state \"Active: 100% %\" as 1"),
        "{diagram}"
    );
    assert!(
        diagram.contains("0 --> 1 : Retry { max, 3 }, % %now"),
        "{diagram}"
    );
    assert!(diagram.contains("note right of 1 : (unnamed)"), "{diagram}");
}

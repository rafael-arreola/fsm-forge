---
name: fsm-forge
description: Build and test state machines in Rust using the `fsm-forge` crate (`Machine`, `Edge`, `Rule`, `Scope`, `Engine`).
---

# State machines with `fsm-forge`

`fsm-forge` is a pure, dependency-free state machine engine. It takes an entity, an event, and an input, runs checks and mutations on a clone, and returns an `Outcome` with the updated entity and effects to run after persisting.

## Rules

- Implement `Machine` on a unit marker type (`pub struct OrderMachine;`), never on `Entity`. `Entity::clone` must produce an independent value (no shared interior mutability).
- Declare one type per transition and implement `Edge<M>` on it (topology constants plus optional `check`, `apply`, `verify`, `effects`, and `name` methods).
- Register `Edge` types with `.edge(MyEdge)`. For global scoped logic, either register a `Rule<M>` struct via `.rule(scope, MyRule)` or attach inline closures directly on `EngineBuilder<M>` via `.check`, `.apply`, `.verify`, `.effects`, or `.emit`.
- Keep edges and rules pure: no I/O, no `async`. Preload external data into `Entity` or `Input`. Describe side effects as values in `effects` and run them in the application layer after saving `outcome.entity`.
- `check` and `Scope::when` see `step.entity` **before** mutation. Use `verify(&self, step, next)` to check invariants on the mutated entity and destination state.
- Move `outcome.entity` and `outcome.effects` directly as independent public fields (`ticket = outcome.entity; for fx in outcome.effects { ... }`), or call `outcome.effect_values()` before moving fields.

## Primitives

| Piece              | Signature / API                                                                                                                                                                                                                                                             | Behavior                                                                                                                                                                                                                                                                  |
| :----------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Machine`          | `State`, `Event`: `Copy + Eq + Hash + Debug + Send + Sync + 'static`<br>`Entity: Clone`, `Input`, `Effect`<br>`Error: Debug + Display + Send + Sync + 'static`<br>`fn state(&Entity) -> State`<br>`fn set_state(&mut Entity, State)`                                        | Domain contract. Optional `state_name` and `event_name` customize labels in error `Display` and Mermaid diagrams.                                                                                                                                                         |
| `Edge<M>`          | `const EVENT: M::Event`<br>`const FROM: &'static [M::State]`<br>`const TO: M::State`<br>Optional: `check`, `apply`, `verify`, `effects`, `name`                                                                                                                             | Fixed topology and optional logic of one transition. `FROM: &[]` makes the edge reachable only via `execute_from_any` / `diagnose_from_any`.                                                                                                                              |
| `Rule<M>`          | `check(&self, step) -> Result<(), M::Error>`<br>`apply(&self, step, &mut entity)`<br>`verify(&self, step, next) -> Result<(), M::Error>`<br>`effects(&self, step, next) -> Vec<M::Effect>`<br>`name(&self) -> &'static str`                                                 | Shared logic struct registered with `.rule(scope, rule)`. Execution order: edge first, then matching scoped rules in registration order. If any `check` or `verify` fails, all errors in that phase are collected into `Rejection::Rules` and mutations are discarded.    |
| `Step<'a, M>`      | `entity: &'a M::Entity` (pre-mutation), `event`, `from`, `to`, `input: &'a M::Input`, `from_any: bool`, `is_self_loop()`                                                                                                                                                    | Read-only context passed to scopes and transition phases.                                                                                                                                                                                                                 |
| `Scope<M>`         | `always()`, `on(&'static [Event])`, `entering(&'static [State])`, `leaving(&'static [State])`, `when(\|step\| bool)`                                                                                                                                                        | Selects transitions for global rules. `entering` and `leaving` **exclude self-loops** (`from == to`); use `Scope::when(\|s\| s.from == State::X)` to include self-loops.                                                                                                  |
| `EngineBuilder<M>` | `.edge(E)`, `.rule(Scope, R)`, `.check(Scope, name, fn)`, `.apply(Scope, name, fn)`, `.verify(Scope, name, fn)`, `.effects(Scope, name, fn)`, `.emit(Scope, name, effect)`, `.initial(State)`, `.roots(iter)`, `.require_events(iter)`, `.require_states(iter)`, `.build()` | Validates at startup and returns `Result<Engine<M>, BuildErrors<M>>`. Inline closure methods infer `M` automatically without turbofish. Reports `Conflict`, `MissingEvent`, `MissingState`, `AmbiguousOriginless`, `ShadowedOriginless`, and `UnreachableState` together. |
| `Engine<M>`        | `execute`, `execute_from_any`, `diagnose`, `diagnose_from_any`, `available` (`-> &[M::Event]`), `available_detailed`, `target`, `initial`, `roots`, `to_mermaid`                                                                                                            | Immutable runtime engine. `diagnose` runs `check`, `apply`, and `verify` without calling `effects`.                                                                                                                                                                       |

## Module layout

For machines with multiple transitions, split domain assembly from edges:

```text
src/<domain>/
  machine.rs     # State, Event, Entity, Input, Effect, Error, impl Machine, engine builder fn
  edges/
    mod.rs       # pub use each edge struct
    <edge>.rs    # one struct + impl Edge<M> per file
```

## Examples

### 1. Machine, edges, and builder (`machine.rs` + `edges/`)

```rust
use fsm_forge::{BuildErrors, Edge, Engine, Machine, Scope, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status { Draft, Open, Closed }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event { Submit, Close, Timeout, ForceClose }

#[derive(Debug, Clone)]
pub struct Ticket {
    pub status: Status,
    pub description: Option<String>,
    pub closed_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub struct Command {
    pub is_supervisor: bool,
    pub inactive_hours: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect { NotifyOpened, PublishUpdate, AuditOverride }

pub struct TicketMachine;

impl Machine for TicketMachine {
    type State = Status;
    type Event = Event;
    type Entity = Ticket;
    type Input = Command;
    type Effect = Effect;
    type Error = &'static str;

    fn state(t: &Ticket) -> Status { t.status }
    fn set_state(t: &mut Ticket, s: Status) { t.status = s; }
}

// Standard edge with check and effects
pub struct Submit;

impl Edge<TicketMachine> for Submit {
    const EVENT: Event = Event::Submit;
    const FROM: &'static [Status] = &[Status::Draft];
    const TO: Status = Status::Open;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        if step.entity.description.is_some() { Ok(()) } else { Err("missing description") }
    }

    fn effects(&self, _step: &Step<'_, TicketMachine>, _next: &Ticket) -> Vec<Effect> {
        vec![Effect::NotifyOpened]
    }
}

// Edge that rejects from-any overrides
pub struct Close;

impl Edge<TicketMachine> for Close {
    const EVENT: Event = Event::Close;
    const FROM: &'static [Status] = &[Status::Open];
    const TO: Status = Status::Closed;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        if !step.from_any { Ok(()) } else { Err("Close cannot be overridden") }
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.closed_reason = Some("resolved");
    }
}

// Configurable edge (fields on the struct)
pub struct Timeout { pub hours: u32 }

impl Edge<TicketMachine> for Timeout {
    const EVENT: Event = Event::Timeout;
    const FROM: &'static [Status] = &[Status::Open];
    const TO: Status = Status::Closed;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        if step.input.inactive_hours >= self.hours { Ok(()) } else { Err("not inactive long enough") }
    }
}

// Originless edge: only reachable via execute_from_any
pub struct ForceClose;

impl Edge<TicketMachine> for ForceClose {
    const EVENT: Event = Event::ForceClose;
    const FROM: &'static [Status] = &[];
    const TO: Status = Status::Closed;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        if step.input.is_supervisor { Ok(()) } else { Err("supervisor required") }
    }

    fn apply(&self, _step: &Step<'_, TicketMachine>, ticket: &mut Ticket) {
        ticket.closed_reason = Some("force closed");
    }
}

pub fn ticket_engine() -> Result<Engine<TicketMachine>, BuildErrors<TicketMachine>> {
    Engine::<TicketMachine>::builder()
        .edge(Submit)
        .edge(Close)
        .edge(Timeout { hours: 72 })
        .edge(ForceClose)
        .emit(Scope::always(), "publish_update", Effect::PublishUpdate)
        .emit(Scope::when(|s| s.from_any), "audit_override", Effect::AuditOverride)
        .initial(Status::Draft)
        .require_events([Event::Submit, Event::Close, Event::Timeout, Event::ForceClose])
        .require_states([Status::Draft, Status::Open, Status::Closed])
        .build()
}
```

### 2. Execution, diagnosis, and error handling

```rust
use fsm_forge::{Phase, Rejection};

fn run_workflow() -> Result<(), Rejection<TicketMachine>> {
    let engine = ticket_engine().expect("valid machine");
    let mut ticket = Ticket {
        status: Status::Draft,
        description: Some("broken login".into()),
        closed_reason: None,
    };
    let cmd = Command::default();

    // Persist `outcome.entity` first, then consume `outcome.effects` by value
    let outcome = engine.execute(&ticket, Event::Submit, &cmd)?;
    ticket = outcome.entity;
    let effects: Vec<Effect> = outcome.effects.into_iter().map(|fx| fx.effect).collect();
    assert_eq!(ticket.status, Status::Open);
    assert_eq!(effects, vec![Effect::NotifyOpened, Effect::PublishUpdate]);

    // Query available transitions and why any would fail
    for (event, verdict) in engine.available_detailed(&ticket, &cmd) {
        match verdict {
            Ok(transition) => println!("{event:?} -> {:?}", transition.to),
            Err(rejection) => println!("{event:?} blocked: {rejection}"),
        }
    }

    // Inspect structured rule rejections
    let err = engine.execute(&ticket, Event::Timeout, &cmd).unwrap_err();
    if let Some(applied_errors) = err.rules() {
        assert_eq!(applied_errors[0].rule, "Timeout");
        assert_eq!(applied_errors[0].phase, Phase::Check);
        assert_eq!(applied_errors[0].error, "not inactive long enough");
    }

    // From-any execution (selects originless `ForceClose` edge)
    let supervisor_cmd = Command { is_supervisor: true, inactive_hours: 0 };
    let outcome = engine.execute_from_any(&ticket, Event::ForceClose, &supervisor_cmd)?;
    assert!(outcome.transition.from_any);
    assert_eq!(outcome.entity.status, Status::Closed);
    Ok(())
}
```

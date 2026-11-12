# fsm-forge

A generic state-machine engine for Rust. It knows nothing about your domain: you define
the states, the events, the entity, and the logic; the engine handles the topology,
execution order, validation, and diagnostics.

- No dependencies.
- No IO: it is pure and deterministic.
- Rules do not run effects: they **describe** them, and the engine returns them so you
  can run them after saving.
- The machine is validated at build time (`build()`), not in production.

It is the state kernel you embed in your service: the graph is declared in code and
validated at startup, and the engine returns the new entity plus the effects.
Persistence, transactions, running the effects, timers, and async runtimes stay in your
application.

## Installation

```sh
cargo add fsm-forge
```

Requires Rust 1.90 or later (edition 2024) and has no dependencies.

## Minimal example

The snippets in this README are illustrative. The runnable examples walk step by step
from `examples/01_minimal/` (smallest) to `examples/08_ticket/` (largest):

```sh
cargo run --example 01_minimal
cargo run --example 03_counter
cargo run --example 08_ticket
cargo run --example 08_ticket -- --mermaid
```

```rust
use fsm_forge::{Edge, Engine, Machine, Scope, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Status { Draft, Issued }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Event { Issue }

#[derive(Clone)]
struct Invoice { status: Status, total: i64 }

struct InvoiceMachine;

impl Machine for InvoiceMachine {
    type State = Status;
    type Event = Event;
    type Entity = Invoice;
    type Input = i64;           // what arrives with the event
    type Effect = &'static str; // what must run afterwards
    type Error = String;        // why it is rejected

    fn state(invoice: &Invoice) -> Status { invoice.status }
    fn set_state(invoice: &mut Invoice, status: Status) { invoice.status = status; }
}

struct Issue;

impl Edge<InvoiceMachine> for Issue {
    const EVENT: Event = Event::Issue;
    const FROM: &'static [Status] = &[Status::Draft];
    const TO: Status = Status::Issued;

    fn check(&self, step: &Step<'_, InvoiceMachine>) -> Result<(), String> {
        if step.entity.total > 0 { Ok(()) } else { Err("total must be positive".into()) }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::<InvoiceMachine>::builder()
        .edge(Issue)
        .emit(Scope::always(), "audit", "audit")
        .require_events([Event::Issue])
        .require_states([Status::Draft, Status::Issued])
        .build()?;

    let mut invoice = Invoice { status: Status::Draft, total: 100 };
    let outcome = engine.execute(&invoice, Event::Issue, &100)?;
    invoice = outcome.entity;

    assert_eq!(invoice.status, Status::Issued);
    Ok(())
}
```

## Architecture

The crate works in two moments. At **build time**, once at startup, you declare the
machine on an `EngineBuilder` and `build()` validates it. At **run time**, on every
request, your application asks the immutable `Engine` to execute or diagnose events, then
persists the result and runs the effects itself:

```mermaid
flowchart LR
    subgraph domain["Your domain"]
        M["Machine<br/>State, Event, Entity,<br/>Input, Effect, Error"]
        E["Edge types<br/>impl Edge: EVENT, FROM, TO,<br/>and optional phases"]
        R["Scoped rules<br/>impl Rule, or builder<br/>.check/.apply/.verify/.effects/.emit"]
        S["Scope<br/>always, on, entering,<br/>leaving, when"]
    end

    subgraph buildtime["Build time: once, at startup"]
        B["EngineBuilder<br/>.edge, .rule, .check, .apply, .verify,<br/>.effects, .emit, .initial, .roots,<br/>.require_events, .require_states"]
        V{"build()"}
        BE["BuildErrors<br/>Conflict, AmbiguousOriginless,<br/>ShadowedOriginless, MissingEvent,<br/>MissingState, UnreachableState"]
        EN["Engine<br/>immutable and reusable"]
    end

    subgraph runtime["Run time: on every request"]
        APP["Your application layer"]
        DB[("Your database")]
        EXT["Queues, email, SLA..."]
    end

    M -.->|"fixes the types"| B
    E -->|".edge(...)"| B
    S -->|"decides where"| R
    R -->|".rule / .check / .emit"| B
    B --> V
    V -->|"Err: all errors together"| BE
    V -->|"Ok"| EN

    APP -->|"execute, execute_from_any"| EN
    EN -->|"Outcome: transition, entity, effects<br/>or Rejection"| APP
    APP -->|"diagnose, diagnose_from_any, available_detailed"| EN
    EN -->|"Verdict (or Vec per event)"| APP
    APP -->|"available, target, initial, roots, to_mermaid"| EN
    APP -->|"first: save outcome.entity"| DB
    APP -->|"then: run the effects"| EXT
```

The engine never touches the database or the outside world: it receives an entity and
returns a new one plus a description of the work to do.

## Concepts

### Edges

An edge is a type: one type per transition. It implements `Edge`, with the topology as
constants and the logic as optional methods. Nothing is hidden in a builder chain.

```rust
struct Issue;

impl Edge<InvoiceMachine> for Issue {
    const EVENT: Event = Event::Issue;
    const FROM: &'static [Status] = &[Status::Draft];
    const TO: Status = Status::Issued;

    fn check(&self, step: &Step<'_, InvoiceMachine>) -> Result<(), String> { /* ... */ }
    fn apply(&self, step: &Step<'_, InvoiceMachine>, invoice: &mut Invoice) { /* ... */ }
    fn verify(&self, step: &Step<'_, InvoiceMachine>, invoice: &Invoice) -> Result<(), String> { /* ... */ }
    fn effects(&self, step: &Step<'_, InvoiceMachine>, invoice: &Invoice) -> Vec<&'static str> { /* ... */ }
}
```

Register it with `.edge(Issue)`. The constants describe the graph and the methods describe
the work:

| Piece     | Kind     | Meaning                                                                        |
| --------- | -------- | ------------------------------------------------------------------------------ |
| `EVENT`   | constant | The event that triggers the edge.                                              |
| `FROM`    | constant | Origin states. `&[]` makes the edge reachable only through `execute_from_any`. |
| `TO`      | constant | Destination state.                                                             |
| `check`   | method   | Precondition; `Err` rejects the transition.                                    |
| `apply`   | method   | Change to the entity; the engine runs it on a copy.                            |
| `verify`  | method   | Post-condition over the modified entity and the destination state.             |
| `effects` | method   | Work the caller must run after persisting.                                     |
| `name`    | method   | Label in diagnostics; defaults to the type name.                               |

Every phase method is optional. An edge without logic only declares the three constants:

```rust
struct Archive;

impl Edge<InvoiceMachine> for Archive {
    const EVENT: Event = Event::Archive;
    const FROM: &'static [Status] = &[Status::Issued];
    const TO: Status = Status::Archived;
}
```

`Edge` and `Rule` are separate traits: `.edge(...)` only accepts `Edge` types and
`.rule(scope, ...)` only accepts `Rule` types, so an edge can never be registered as a
global rule by mistake.

- An event can have **several edges** if they leave from different states.
- The same `(event, origin)` **cannot** have two targets: `build()` reports it as
  `Conflict`.
- `FROM: &[A, B]` declares several origins.
- `FROM: &[]` declares an edge without origin: `execute` never selects it, only
  `execute_from_any` does.

The type may carry configuration in its fields; the engine uses the instance you pass to
`.edge(...)` on every execution:

```rust
struct InactivityTimeout {
    hours: u32,
}

impl Edge<TicketMachine> for InactivityTimeout {
    // EVENT, FROM, TO

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        if step.input.inactive_hours >= self.hours { Ok(()) } else { Err("too early") }
    }
}

.edge(InactivityTimeout { hours: 72 })
```

### Entry point

The engine does not create entities. Declare the states they are born in:

```rust
.initial(Status::Draft)                    // "state zero"
.roots([Status::Draft, Status::Imported])  // several legitimate entries
```

```rust
let initial = engine.initial(); // Option<State>: the primary root
let roots = engine.roots();     // &[State]
```

This is **metadata**: it does not affect `execute`, `available`, or edge selection.

What it provides:

- `build()` checks that every state in the topology is reachable from some root and
  reports `UnreachableState` for the ones that are not.
- The diagram marks each root with `[*] --> State`.
- `engine.initial()` tells the application layer which state a new entity starts in.

What it **does not** do:

- It does not create entities or assign state: `execute` always reads the state the
  entity already carries.
- It does not restrict transitions: an entity in a non-initial state behaves the same.
- It does not force any entity to start there: it is a declaration, not a runtime
  guarantee.

Reachability validation only runs when roots are declared; without them,
"reachable" would have no definition. An edge without origin counts for reachability as
long as at least one reachable state has no explicit edge for that event (otherwise
`build()` reports `ShadowedOriginless`). A root that appears in no edge is valid.

If you need runtime behavior ("this may only run from the initial state"), that is a
domain rule with `check`, not an engine property.

### Rules

A `Rule` is logic without topology, registered with a `Scope` so it runs for every
transition the scope selects. It exposes the exact same optional phases as `Edge`; the
engine runs the edge first, then every matching scoped rule in registration order.

| If the logic...                                               | Put it in                     |
| ------------------------------------------------------------- | ----------------------------- |
| depends on the specific event, origin, or destination         | the `Edge` impl               |
| applies to many transitions by event, state, or from-any mode | a standalone `Rule` + `Scope` |
| describes work the caller must run after saving               | `effects`, in either of them  |

A `Rule` implements the methods it needs; all of them are optional:

| Method                      | When it runs                                      | Purpose             |
| --------------------------- | ------------------------------------------------- | ------------------- |
| `check(&step)`              | Before touching the entity                        | Precondition        |
| `apply(&step, &mut entity)` | On a copy of the entity                           | Changes             |
| `verify(&step, &next)`      | On the already modified entity with the new state | Postcondition       |
| `effects(&step, &next)`     | At the end, without mutating anything             | Describe later work |

`Step` is the context: `entity` (before the change), `event`, `from`, `to`, `input`, and
`from_any`. It includes `is_self_loop()`.

For single-phase global rules you do not need a struct: `EngineBuilder` registers closures
directly with `.check(scope, name, ...)`, `.apply(scope, name, ...)`,
`.verify(scope, name, ...)`, `.effects(scope, name, ...)`, and `.emit(scope, name, effect)`.
Because the builder already carries `M`, closures infer `step.entity`, `step.input`, and
`next` without type annotations.

### Scopes

To avoid repeating a rule on every edge, register a `Rule` type with `.rule(scope, ...)` or
an inline phase method on `EngineBuilder`:

```rust
.emit(Scope::always(), "publish_update", Effect::PublishUpdate)
.check(Scope::on(&[Event::Issue]), "require_notes", |step| { /* ... */ })
.rule(Scope::entering(&[Status::Issued]), SendInvoice)
.rule(Scope::leaving(&[Status::Issued]), CloseReceivable)
.emit(Scope::when(|step| step.from_any), "override_audit", Effect::AuditOverride)
```

`entering` and `leaving` **exclude self-transitions**. If a rule must run on every
transition out of a state, a `State -> State` retry included, write the condition with
`Scope::when`:

```rust
.rule(Scope::when(|step| step.from == Status::Issued), AuditIssued)
```

### Large catalogs: sections

One type per transition keeps the catalog short: each line is a complete, named
transition, and the logic lives in its `impl`. With many edges, group them into
functions that take and return the builder:

```rust
fn creation(builder: EngineBuilder<ShipmentMachine>) -> EngineBuilder<ShipmentMachine> {
    builder.edge(NewDraft).edge(DraftConfirmed)
}

fn picking(builder: EngineBuilder<ShipmentMachine>) -> EngineBuilder<ShipmentMachine> {
    builder.edge(ToPickup).edge(DriverAssignedToPickup)
}

let engine = picking(creation(Engine::<ShipmentMachine>::builder()))
    .edge(...)
    .build()?;
```

`Edge` types are static: `EVENT`, `FROM`, and `TO` are constants, so one type maps to
one edge. When the same logic applies to several events, write it once as a standalone
`Rule` and register it with a scope:

```rust
struct RequireDriverId;

impl Rule<ShipmentMachine> for RequireDriverId {
    fn check(&self, step: &Step<'_, ShipmentMachine>) -> Result<(), &'static str> {
        /* ... */
    }
}

.rule(
    Scope::on(&[Ev::DriverAssignedToPickup, Ev::DriverReassignedToPickup]),
    RequireDriverId,
)
```

## How execution works

`execute(&entity, event, &input)` always follows the same order:

1. **Select the edge** for the event from the current state, or the from-any candidate
   when `execute_from_any` is used.
2. **Collect the rules**: first the edge's own logic, then the global rules whose scope
   applies, in registration order.
3. **Run every `check`**. If any fails, reject with **all** errors together.
4. **Clone the entity**, run every `apply` in order, and move the state.
5. **Run every `verify`** on the resulting entity. If any fails, everything is discarded
   and there is no `Outcome`.
6. **Collect the `effects`** and return the `Outcome`.

The same order, as a sequence:

```mermaid
sequenceDiagram
    autonumber
    participant App as Your application
    participant Eng as Engine
    participant Edge as Edge logic
    participant Rules as Scoped rules

    App->>Eng: execute(entity, event, input)
    Eng->>Eng: select the edge from Machine::state(entity) and the event
    alt no edge applies
        Eng-->>App: Err: UnknownEvent, NotAllowed, or Ambiguous
    end
    Eng->>Eng: build the Step and collect the edge plus the matching scoped rules
    Eng->>Edge: check(step)
    Eng->>Rules: check(step), in registration order
    alt any check fails
        Eng-->>App: Err: Rejection::Rules with every error, phase Check
    end
    Eng->>Eng: next = entity.clone()
    Eng->>Edge: apply(step, next)
    Eng->>Rules: apply(step, next)
    Eng->>Eng: Machine::set_state(next, to)
    Eng->>Edge: verify(step, next)
    Eng->>Rules: verify(step, next)
    alt any verify fails
        Eng-->>App: Err: Rejection::Rules, phase Verify, next is discarded
    end
    Eng->>Edge: effects(step, next)
    Eng->>Rules: effects(step, next)
    Eng-->>App: Ok: Outcome with transition, entity = next, effects
    App->>App: save outcome.entity, then run the effects
    Note over App,Eng: diagnose follows the same steps up to verify and stops before effects. It returns the Transition, without entity or effects
```

The original entity is never modified: if the execution is rejected, nothing changes.

> **`check` sees the previous entity, not the result.** All `check`s run before any
> `apply`, so a global rule that looks like an invariant does not protect the final
> state. That is what `verify` is for: it runs after mutating and after moving the state.
>
> **Assign the result.** `execute` returns the new entity in `Outcome::entity`; the
> engine does not mutate the original. If you forget `*entity = outcome.entity`, the
> transition was evaluated and the effects were produced, but your data did not change.

### Outcome

```rust
pub struct Outcome<M: Machine> {
    pub transition: Transition<M>,         // what ran (see below)
    pub entity: M::Entity,                 // ready to persist
    pub effects: Vec<AppliedEffect<M>>,    // effect + rule name
}

pub struct Transition<M: Machine> {
    pub event: M::Event,
    pub from: M::State,
    pub to: M::State,
    pub rules: Vec<&'static str>,          // which rules participated, in execution order
    pub from_any: bool,
}
```

`Transition` is what an execution and a diagnosis have in common: `execute` returns it
inside the `Outcome`, and `diagnose` returns it alone (see
[Querying without executing](#querying-without-executing)).

Because `Outcome` has public fields and no `Drop` implementation, Rust lets you move each
field independently in the natural order—save `outcome.entity` first, then consume
`outcome.effects` by value—without cloning or declaring `mut outcome`:

```rust
let outcome = engine.execute(&invoice, Event::Issue, &100)?;
invoice = outcome.entity;              // moves only the entity
for applied in outcome.effects {       // moves effects by value afterwards
    dispatch(applied.effect);
}
let to = outcome.transition.to;        // Copy fields stay readable
```

For quick inspection by reference (before moving fields), `outcome.effect_values()` yields
`&M::Effect` without the rule name, and `outcome.transition.is_self_loop()` compares origin
and target. If you prefer to keep the whole `Outcome` intact, clone the entity explicitly
with `entity = outcome.entity.clone()`; the cost stays visible at the call site.

## From-any mode

`execute_from_any(&entity, event, &input)` bypasses the origin-state requirement when no
edge starts at the current state, while still **running all rules**, including `check`.
Edge selection follows these rules:

- If the event has an edge from the current state, that one is used.
- Otherwise, an **edge without origin** for the event is used ("from any state"). More
  than one edge without origin for the same event is rejected at build time with
  `AmbiguousOriginless`.
- Otherwise, the event's only edge is used. If it has several, it is rejected with
  `Ambiguous` instead of guessing.

```mermaid
flowchart TD
    A["event + current state"] --> B{"Does the event<br/>have edges?"}
    B -->|"no"| U["Rejection::UnknownEvent"]
    B -->|"yes"| C{"Is there an edge from<br/>the current state?"}
    C -->|"yes"| OK1["use that edge"]
    C -->|"no"| D{"execute_from_any?"}
    D -->|"no, execute"| N["Rejection::NotAllowed"]
    D -->|"yes"| E{"Is there an edge<br/>without origin?"}
    E -->|"yes"| OK2["use the edge without origin"]
    E -->|"no"| F{"Does the event have<br/>a single edge?"}
    F -->|"yes"| OK3["use that edge"]
    F -->|"no"| G["Rejection::Ambiguous"]
```

An edge with `FROM: &[]` is only reachable this way: `execute` never selects it. Because
every rule still runs, the edge can reject a from-any execution explicitly:

```rust
impl Edge<TicketMachine> for SolutionConfirmed {
    const EVENT: TicketEvent = TicketEvent::SolutionConfirmed;
    const FROM: &'static [TicketStatus] = &[TicketStatus::Resolved];
    const TO: TicketStatus = TicketStatus::Closed;

    fn check(&self, step: &Step<'_, TicketMachine>) -> Result<(), &'static str> {
        if !step.from_any { Ok(()) } else { Err("this transition cannot be overridden") }
    }
}
```

## Querying without executing

```rust
// Can this event run right now?
engine.diagnose(&entity, event, &input)?; // -> Transition

// Same, ignoring the origin state.
engine.diagnose_from_any(&entity, event, &input)?;

// Which events can run, and why not?
for (event, verdict) in engine.available_detailed(&entity, &input) { /* ... */ }
```

`diagnose` follows the same path as `execute`, so an `Ok` guarantees that `execute`
would also accept. It returns the same `Transition` that `execute` puts in
`Outcome::transition` (`event`, `from`, `to`, `rules`, `from_any`), without an entity or
effects.

```rust
pub type Verdict<M> = Result<Transition<M>, Rejection<M>>;
```

## Reading the topology

```rust
let next = engine.target(Event::Issue, Status::Draft); // -> Option<Status>
let open = engine.available(&invoice);                 // -> &[Event]
```

`target` answers where an event goes from a given state according to the declaration, and
`available` lists the events leaving the entity's current state in declaration order,
without evaluating rules. Edges without origin have no `target`; they only run through
`execute_from_any` / `diagnose_from_any`.

## Errors

### At execution: `Rejection<M>`

| Variant                       | When                                                                                                     |
| ----------------------------- | -------------------------------------------------------------------------------------------------------- |
| `UnknownEvent(event)`         | The event has no declared edges                                                                          |
| `NotAllowed { event, from }`  | The event has edges, but none starts at the current state                                                |
| `Ambiguous(event)`            | When overriding, the event has several edges, none starts at the current state, and none has `FROM: &[]` |
| `Rules(Vec<AppliedError<M>>)` | One or more rules rejected; each error carries its rule and phase                                        |

Each `AppliedError` includes:

```rust
pub struct AppliedError<M: Machine> {
    pub rule: &'static str,   // "perception_successful"
    pub phase: Phase,         // Check | Verify
    pub error: M::Error,
}
```

The `Display` message keeps everything:

```text
rejected by rules: available_tiles [check]: not enough movement
```

And you can keep only the errors with `rejection.rule_errors()`, or the full traceability
with `rejection.rules()`, which returns `Option<&[AppliedError<M>]>`
(`None` if the rejection did not come from rules).

`Rejection` and `BuildError` are `#[non_exhaustive]`: an external `match` needs a `_`
arm.

### At build time: `BuildErrors<M>`

`build()` accumulates **all** errors and returns them together:

| Variant                      | When                                                                                  |
| ---------------------------- | ------------------------------------------------------------------------------------- |
| `Conflict { event, from }`   | Two edges for the same event leave the same state                                     |
| `MissingEvent(event)`        | An event in `require_events` has no edge                                              |
| `MissingState(state)`        | A state in `require_states` appears in no edge and is not a root                      |
| `AmbiguousOriginless(event)` | More than one edge without origin for the same event                                  |
| `ShadowedOriginless(event)`  | An edge without origin is shadowed in every reachable state and can never be selected |
| `UnreachableState(state)`    | With roots declared, the state is not reachable from any of them                      |

## Diagram

```rust
let mermaid = engine.to_mermaid(); // stateDiagram-v2
```

The ids are sanitized from the state's `Debug` (disambiguated on collisions or reserved
keywords); the labels come from `Machine::state_name` and `Machine::event_name`. Edges
without origin appear as a note on their target state.

## Guarantees

- **Pure**: no rule receives IO or an async runtime; everything it needs arrives in
  `Entity` or `Input`.
- **Deterministic**: given `(state, event)`, the target is fixed at build time. There is
  no target selection at runtime.
- **Atomic in memory**: if anything is rejected, the original entity is left untouched.
- **No dependencies**: `[dependencies]` is empty.
- **No `unsafe`**: enforced by a lint in `Cargo.toml`.

## Examples

The examples are numbered incrementally (`01_` to `08_`), introducing one capability at a
time. Each one is a directory with the same shape:

```text
examples/08_ticket/
  main.rs        entry point and application layer
  machine.rs     domain types, Machine impl, and engine assembly
  edges/         one file per transition
```

| Example          | Step | One idea                                                                                       |
| ---------------- | ---- | ---------------------------------------------------------------------------------------------- |
| `01_minimal`     | 1    | One `Machine`, one `Edge` (`Idle -> Running`), `execute`, and keeping `outcome.entity`.        |
| `02_phases`      | 2    | `Input` and the four phases (`check`, `apply`, `verify`, `effects`) + persist & dispatch.      |
| `03_counter`     | 3    | `Start -> Middle -> End` with a self-loop (`Increment`) and a scoped `>= 10` check.            |
| `04_scopes`      | 4    | Reusable `Rule` structs and all five `Scope`s (`always`, `on`, `entering`, `leaving`, `when`). |
| `05_diagnostics` | 5    | Querying without executing: `available`, `target`, `diagnose`, and `available_detailed`.       |
| `06_from_any`    | 6    | `execute_from_any`, originless edges (`FROM: &[]`), blocking overrides, and override audit.    |
| `07_topology`    | 7    | Build-time validation (`roots`, `initial`, `BuildErrors`) and `to_mermaid()`.                  |
| `08_ticket`      | 8    | Full support-ticket lifecycle with configurable edges, scopes, effects, and overrides.         |

```sh
cargo run --example 01_minimal
cargo run --example 02_phases
cargo run --example 03_counter
cargo run --example 04_scopes
cargo run --example 05_diagnostics
cargo run --example 06_from_any
cargo run --example 07_topology
cargo run --example 08_ticket
cargo run --example 08_ticket -- --mermaid
```

`07_topology` also builds an intentionally broken engine inside `main.rs` to show how
`build()` reports every structural error together.

## Tests

The integration suite covers the engine from the simplest topology to the full ticket
lifecycle:

| File               | Coverage                                                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------------------------------- |
| `tests/basic.rs`   | Door and traffic light: simple edges, self-loops, several origins, from-any mode, custom labels, Mermaid.           |
| `tests/rules.rs`   | Invoice: edge phases, `Rule` structs, builder closure methods (`.check`/`.apply`/`.verify`/`.effects`), `diagnose`. |
| `tests/scopes.rs`  | Article: all five `Scope`s, self-loop boundaries, from-any scope, registration order.                               |
| `tests/builder.rs` | Shipment: conflicts, missing events/states, ambiguous and shadowed originless edges, reachability, roots.           |
| `tests/ticket.rs`  | Full support-ticket lifecycle: creation, assignment, waiting, reopen, timeout, supervisor override.                 |

Run them with `cargo test`.

## Crate map

All public types are exported at the crate root (`use fsm_forge::{Engine, Machine, Edge, ...};`).

| File             | Contents                                              |
| ---------------- | ----------------------------------------------------- |
| `src/lib.rs`     | Public surface and crate docs                         |
| `src/machine.rs` | `Machine`: the domain contract                        |
| `src/rule.rs`    | `Rule`, `Step`, `Scope`: shared logic and scope       |
| `src/edge.rs`    | `Edge` trait and the compiled edge used by the engine |
| `src/engine.rs`  | `Engine`: execute, diagnose, and query                |
| `src/builder.rs` | `EngineBuilder`: declare, attach rules, and validate  |
| `src/outcome.rs` | `Outcome`, `Transition`, `AppliedEffect`, `Verdict`   |
| `src/error.rs`   | `Rejection`, `BuildError`, `BuildErrors`: what failed |
| `src/mermaid.rs` | `to_mermaid`: the diagram                             |

## Running the checks

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

## License

MIT. See [LICENSE](LICENSE).

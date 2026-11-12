//! # fsm-forge
//!
//! A generic state-machine engine. It knows nothing about your domain: each domain
//! defines its states, events, entity, input, effects, and errors with [`Machine`].
//!
//! ```text
//! edge  = (event, from, to) + phases       -> one transition    impl Edge<M>
//! rule  = phases + scope                   -> many transitions  .rule(Scope, ...) / .check(...)
//! phase = check / apply / verify / effects -> shared pipeline
//! ```
//!
//! An edge declares one transition and its optional logic; a scoped rule shares logic
//! across many transitions. The engine runs the edge first, then every matching rule in
//! registration order.
//!
//! ## Execution pattern
//!
//! Always the same order:
//!
//! 1. select the edge for the event from the current state (or using from-any fallback in
//!    from-any mode)
//! 2. collect the edge logic + the global rules whose [`Scope`] matches
//! 3. `check` on all of them: if any fails, reject with ALL errors
//! 4. `apply` on all of them over a copy of the entity, then move the state
//! 5. `verify` on all of them over the resulting entity: if any fails, reject
//! 6. `effects` on all of them -> [`Outcome`]
//!
//! ## Query without executing
//!
//! [`Engine::diagnose`] runs the same pattern and returns only the verdict, and
//! [`Engine::available_detailed`] applies it to every available event.
//!
//! ## Read the topology
//!
//! [`Engine::target`] exposes the destination of an event from a given state without
//! running rules, and [`Engine::available`] lists the events leaving the entity's current
//! state. [`Engine::to_mermaid`] draws the whole topology.
//!
//! The entry point is declared on the builder with `.initial(...)` or `.roots([...])`
//! and read with [`Engine::initial`] / [`Engine::roots`].
//!
//! It is pure: no IO. Persisting and running effects is up to the caller.
//!
//! ## Example
//!
//! ```
//! use fsm_forge::{Edge, Engine, Machine, Outcome, Scope, Step};
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//! enum Status {
//!     Draft,
//!     Issued,
//! }
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//! enum Event {
//!     Issue,
//! }
//!
//! #[derive(Clone)]
//! struct Invoice {
//!     status: Status,
//! }
//!
//! struct InvoiceMachine;
//!
//! impl Machine for InvoiceMachine {
//!     type State = Status;
//!     type Event = Event;
//!     type Entity = Invoice;
//!     type Input = ();
//!     type Effect = &'static str;
//!     type Error = String;
//!
//!     fn state(invoice: &Invoice) -> Status {
//!         invoice.status
//!     }
//!
//!     fn set_state(invoice: &mut Invoice, status: Status) {
//!         invoice.status = status;
//!     }
//! }
//!
//! struct Issue;
//!
//! impl Edge<InvoiceMachine> for Issue {
//!     const EVENT: Event = Event::Issue;
//!     const FROM: &'static [Status] = &[Status::Draft];
//!     const TO: Status = Status::Issued;
//!
//!     fn check(&self, _step: &Step<'_, InvoiceMachine>) -> Result<(), String> {
//!         Ok(())
//!     }
//! }
//!
//! let engine = Engine::<InvoiceMachine>::builder()
//!     .edge(Issue)
//!     .emit(Scope::always(), "audit", "audit")
//!     .build()
//!     .expect("valid machine");
//!
//! let invoice = Invoice {
//!     status: Status::Draft,
//! };
//! let outcome: Outcome<InvoiceMachine> = engine.execute(&invoice, Event::Issue, &()).unwrap();
//!
//! assert_eq!(outcome.transition.to, Status::Issued);
//! assert_eq!(outcome.effect_values().copied().collect::<Vec<_>>(), vec!["audit"]);
//! ```

mod builder;
mod edge;
mod engine;
mod error;
mod machine;
mod mermaid;
mod outcome;
mod rule;

pub use builder::EngineBuilder;
pub use edge::Edge;
pub use engine::Engine;
pub use error::{AppliedError, BuildError, BuildErrors, Phase, Rejection};
pub use machine::Machine;
pub use outcome::{AppliedEffect, Outcome, Transition, Verdict};
pub use rule::{Rule, Scope, Step};

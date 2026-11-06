//! # fsm-forge
//!
//! A generic state-machine engine. It knows nothing about your domain: each domain
//! defines its states, events, entity, input, effects, and errors with [`Machine`].
//!
//! ```text
//! edge  = (event, from, to) + rule         -> one transition    impl Edge<M> + impl Rule<M>
//! rule  = logic + scope                    -> many transitions  .rule(Scope, ...)
//! phase = check / apply / verify / effects -> shared pipeline
//! ```
//!
//! An edge is a rule with fixed topology; a scoped rule is the shared case. The engine
//! runs the edge first, then every matching rule in registration order.
//!
//! ## Execution pattern
//!
//! Always the same order:
//!
//! 1. select the edge for the event from the current state (or ignoring the origin in
//!    from-any mode)
//! 2. collect the edge logic + the global rules whose [`Scope`] matches
//! 3. [`Rule::check`] on all of them: if any fails, reject with ALL errors
//! 4. [`Rule::apply`] on all of them over a copy of the entity, then move the state
//! 5. [`Rule::verify`] on all of them over the resulting entity: if any fails, reject
//! 6. [`Rule::effects`] on all of them -> [`Outcome`]
//!
//! ## Query without executing
//!
//! [`Engine::diagnose`] runs the same pattern and returns only the verdict, and
//! [`Engine::available_detailed`] applies it to every available event.
//!
//! ## Read the topology
//!
//! [`Engine::target`] exposes the destination of an event from a given state without
//! running rules. [`Engine::to_mermaid`] draws the whole topology.
//!
//! The entry point is declared on the builder with `.initial(...)` or `.roots([...])`
//! and read with [`Engine::initial`] / [`Engine::roots`].
//!
//! It is pure: no IO. Persisting and running effects is up to the caller.
//!
//! ## Example
//!
//! ```
//! use fsm_forge::{Edge, Engine, Machine, Outcome, Rule, Scope, Step, emit};
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
//! }
//!
//! impl Rule<InvoiceMachine> for Issue {
//!     fn check(&self, _step: &Step<'_, InvoiceMachine>) -> Result<(), String> {
//!         Ok(())
//!     }
//! }
//!
//! let engine = Engine::<InvoiceMachine>::builder()
//!     .edge(Issue)
//!     .rule(Scope::always(), emit("audit", "audit"))
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
pub use rule::{NamedRule, Rule, Scope, Step, emit, guard, mutate, verify};

/// Types for declaring, executing, and describing a machine.
///
/// The `guard`, `mutate`, `verify`, and `emit` functions are exported at the crate root
/// but not included here, so a glob import does not clash with domain identifiers.
pub mod prelude {
    pub use crate::{
        AppliedEffect, AppliedError, BuildError, BuildErrors, Edge, Engine, EngineBuilder, Machine,
        NamedRule, Outcome, Phase, Rejection, Rule, Scope, Step, Transition, Verdict,
    };
}

//! The edge contract: the topology as constants; the logic comes from [`Rule`].

use crate::machine::Machine;
use crate::rule::Rule;

/// A declared edge: one event, its origins, and its destination.
///
/// An edge is a [`Rule`] with fixed topology. Declare one type per transition and
/// implement both traits on it: `Edge` for the topology (constants) and [`Rule`] for the
/// logic (methods, all optional). An edge without logic implements [`Rule`] with an empty
/// block:
///
/// ```
/// # use fsm_forge::{Edge, Machine, Rule, Step};
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum Status { Draft, Issued, Archived }
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum Event { Issue, Archive }
/// # #[derive(Clone)]
/// # struct Invoice { status: Status, total: i64 }
/// # struct InvoiceMachine;
/// # impl Machine for InvoiceMachine {
/// #     type State = Status;
/// #     type Event = Event;
/// #     type Entity = Invoice;
/// #     type Input = ();
/// #     type Effect = ();
/// #     type Error = &'static str;
/// #     fn state(invoice: &Invoice) -> Status { invoice.status }
/// #     fn set_state(invoice: &mut Invoice, status: Status) { invoice.status = status; }
/// # }
/// struct Issue;
///
/// impl Edge<InvoiceMachine> for Issue {
///     const EVENT: Event = Event::Issue;
///     const FROM: &'static [Status] = &[Status::Draft];
///     const TO: Status = Status::Issued;
/// }
///
/// impl Rule<InvoiceMachine> for Issue {
///     fn check(&self, step: &Step<'_, InvoiceMachine>) -> Result<(), &'static str> {
///         if step.entity.total > 0 { Ok(()) } else { Err("total must be positive") }
///     }
/// }
///
/// struct Archive;
///
/// impl Edge<InvoiceMachine> for Archive {
///     const EVENT: Event = Event::Archive;
///     const FROM: &'static [Status] = &[Status::Issued];
///     const TO: Status = Status::Archived;
/// }
///
/// impl Rule<InvoiceMachine> for Archive {}
/// ```
///
/// Register it with [`EngineBuilder::edge`](crate::EngineBuilder::edge). The type may
/// carry configuration in its fields; the engine stores the instance when the machine is
/// built and uses it on every execution.
///
/// When the same logic applies to many transitions, declare it once as a standalone
/// [`Rule`] with a [`Scope`](crate::Scope) instead of repeating it in every edge.
///
/// # Register it with `.edge(...)`, never with `.rule(...)`
///
/// Because an edge is a [`Rule`], the compiler also accepts it in
/// [`EngineBuilder::rule`](crate::EngineBuilder::rule), and `build()` does not report it.
/// There, its logic would run on every transition the scope selects, not only on its own:
/// a `check` would reject unrelated transitions, an `apply` would modify the entity on
/// each of them, and its `effects` would be emitted everywhere. An `apply`-only edge is
/// the hardest case to notice, because nothing fails.
pub trait Edge<M: Machine>: Rule<M> {
    /// The event that triggers this edge.
    const EVENT: M::Event;

    /// Origin states. Empty means the edge is only reachable through
    /// `execute_from_any`.
    const FROM: &'static [M::State];

    /// Destination state.
    const TO: M::State;
}

/// The compiled edge the engine stores: the topology plus the edge itself as its rule.
pub(crate) struct CompiledEdge<M: Machine> {
    pub(crate) event: M::Event,
    pub(crate) from: Vec<M::State>,
    pub(crate) to: M::State,
    pub(crate) rule: Box<dyn Rule<M>>,
}

impl<M: Machine> CompiledEdge<M> {
    pub(crate) fn new<E: Edge<M>>(edge: E) -> Self {
        // A repeated state in the declaration is the same origin, not a conflict.
        let mut from: Vec<M::State> = Vec::new();
        for state in E::FROM {
            if !from.contains(state) {
                from.push(*state);
            }
        }
        Self {
            event: E::EVENT,
            from,
            to: E::TO,
            rule: Box::new(edge),
        }
    }
}

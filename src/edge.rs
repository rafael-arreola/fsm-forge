//! The edge contract: topology constants and optional transition phases in one trait.

use crate::machine::Machine;
use crate::rule::{Rule, Step, short_type_name};

/// A declared edge: one event, its origins, its destination, and its optional logic.
///
/// Declare one type per transition and implement `Edge` on it: the constants fix the
/// topology and the methods (all optional) implement its logic:
///
/// ```
/// # use fsm_forge::{Edge, Machine, Step};
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
///
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
/// ```
///
/// Register it with [`EngineBuilder::edge`](crate::EngineBuilder::edge). The type may
/// carry configuration in its fields; the engine stores the instance when the machine is
/// built and uses it on every execution.
///
/// When the same logic applies to many transitions, declare it once as a standalone
/// [`Rule`] with a [`Scope`](crate::Scope) and register it via
/// [`EngineBuilder::rule`](crate::EngineBuilder::rule). Because `Edge` and [`Rule`] are
/// distinct traits, the compiler prevents accidentally registering an edge as a global
/// rule.
pub trait Edge<M: Machine>: Send + Sync + 'static {
    /// The event that triggers this edge.
    const EVENT: M::Event;

    /// Origin states. Empty means the edge is only reachable through
    /// `execute_from_any` or `diagnose_from_any`.
    const FROM: &'static [M::State];

    /// Destination state.
    const TO: M::State;

    /// Precondition. `Err` rejects the transition; the errors of all rules are collected.
    fn check(&self, _step: &Step<'_, M>) -> Result<(), M::Error> {
        Ok(())
    }

    /// Changes to the entity. The engine moves the state after every rule has run.
    fn apply(&self, _step: &Step<'_, M>, _entity: &mut M::Entity) {}

    /// Post-condition over the already-modified entity, with the destination state set.
    /// `Err` rejects the whole transition: no `Outcome` is produced.
    fn verify(&self, _step: &Step<'_, M>, _next: &M::Entity) -> Result<(), M::Error> {
        Ok(())
    }

    /// Effects the caller must run after persisting.
    ///
    /// `next` is the modified entity, so an effect can use computed values.
    fn effects(&self, _step: &Step<'_, M>, _next: &M::Entity) -> Vec<M::Effect> {
        Vec::new()
    }

    /// Name shown in [`Transition::rules`](crate::Transition::rules), in effects, and in
    /// rejections. Defaults to the type name without module or generics.
    fn name(&self) -> &'static str {
        short_type_name::<Self>()
    }
}

/// Adapts an [`Edge`] into the internal [`Rule`] pipeline without making `Edge` a subtrait
/// of `Rule` in the public API.
struct EdgeRule<E>(E);

impl<M: Machine, E: Edge<M>> Rule<M> for EdgeRule<E> {
    fn check(&self, step: &Step<'_, M>) -> Result<(), M::Error> {
        self.0.check(step)
    }

    fn apply(&self, step: &Step<'_, M>, entity: &mut M::Entity) {
        self.0.apply(step, entity);
    }

    fn verify(&self, step: &Step<'_, M>, next: &M::Entity) -> Result<(), M::Error> {
        self.0.verify(step, next)
    }

    fn effects(&self, step: &Step<'_, M>, next: &M::Entity) -> Vec<M::Effect> {
        self.0.effects(step, next)
    }

    fn name(&self) -> &'static str {
        self.0.name()
    }
}

/// The compiled edge the engine stores: the topology plus the edge wrapped as a rule.
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
            rule: Box::new(EdgeRule(edge)),
        }
    }
}

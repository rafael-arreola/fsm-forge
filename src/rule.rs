//! The logic of a transition: [`Rule`], and where it runs.
//!
//! A [`Rule`] is logic without topology. It runs in two places:
//!
//! - on an [`Edge`](crate::Edge) type, for that transition only: the edge's constants fix
//!   its event, origins, and destination;
//! - registered with a [`Scope`], for every transition the scope selects.
//!
//! [`Step`] is the context it receives. The engine runs the edge's rule first, then every
//! matching scoped rule in registration order, and merges their phases: `check`, `apply`,
//! `verify`, `effects`.
//!
//! Use a scoped rule when the same logic applies to many transitions:
//!
//! - an effect that must run on every transition, or whenever entering a state;
//! - a precondition shared by several events, such as `retroactive` or "the record is
//!   locked";
//! - an audit field updated on every accepted transition.
//!
//! If the logic depends on the specific event or on a concrete origin/destination, put it
//! in the [`Edge`](crate::Edge) instead.
//!
//! For the common case you do not need to implement the trait: [`guard`], [`mutate`],
//! [`verify`], and [`emit`] build named rules from closures.

use std::fmt;

use crate::machine::Machine;

/// The logic of a transition. Every method is optional.
///
/// An [`Edge`](crate::Edge) is a `Rule` with fixed topology: the edge type implements this
/// trait for its own logic. Registered with a [`Scope`], the same trait describes logic
/// shared by many transitions. Both run through the same phases.
///
/// A type that implements [`Edge`](crate::Edge) is registered with
/// [`EngineBuilder::edge`](crate::EngineBuilder::edge), never with
/// [`EngineBuilder::rule`](crate::EngineBuilder::rule): see the warning on
/// [`Edge`](crate::Edge).
pub trait Rule<M: Machine>: Send + Sync + 'static {
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

/// The type name without module path or generic arguments: `app::Audit<app::Invoice>`
/// becomes `Audit`.
pub(crate) fn short_type_name<T: ?Sized>() -> &'static str {
    let full = std::any::type_name::<T>();
    // The generics go first: their arguments carry module paths of their own, so cutting
    // at the last `::` first would keep the argument (`Invoice>`) instead of the type.
    let without_generics = full.split('<').next().unwrap_or(full);
    without_generics
        .rsplit("::")
        .next()
        .unwrap_or(without_generics)
}

/// The execution in progress: everything a rule or a scope can see.
pub struct Step<'a, M: Machine> {
    /// Entity BEFORE the change.
    pub entity: &'a M::Entity,

    /// Event being executed.
    pub event: M::Event,

    /// Origin state.
    pub from: M::State,

    /// Destination state.
    pub to: M::State,

    /// Data that arrived with the event.
    pub input: &'a M::Input,

    /// `true` when the transition runs through `Engine::execute_from_any`.
    pub from_any: bool,
}

impl<M: Machine> Step<'_, M> {
    /// `true` when origin and destination are the same state.
    #[must_use]
    pub fn is_self_loop(&self) -> bool {
        self.from == self.to
    }
}

impl<M: Machine> Clone for Step<'_, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Machine> Copy for Step<'_, M> {}

impl<M: Machine> fmt::Debug for Step<'_, M>
where
    M::Entity: fmt::Debug,
    M::Input: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Step")
            .field("entity", &self.entity)
            .field("event", &M::event_name(self.event))
            .field("from", &M::state_name(self.from))
            .field("to", &M::state_name(self.to))
            .field("input", &self.input)
            .field("from_any", &self.from_any)
            .finish()
    }
}

type Predicate<M> = dyn Fn(&Step<'_, M>) -> bool + Send + Sync;
type CheckFn<M> = dyn Fn(&Step<'_, M>) -> Result<(), <M as Machine>::Error> + Send + Sync;
type ApplyFn<M> = dyn Fn(&Step<'_, M>, &mut <M as Machine>::Entity) + Send + Sync;
type VerifyFn<M> = dyn Fn(&Step<'_, M>, &<M as Machine>::Entity) -> Result<(), <M as Machine>::Error>
    + Send
    + Sync;
type EffectsFn<M> =
    dyn Fn(&Step<'_, M>, &<M as Machine>::Entity) -> Vec<<M as Machine>::Effect> + Send + Sync;

/// Where a global rule applies.
#[must_use]
pub struct Scope<M: Machine>(ScopeKind<M>);

/// The static cases avoid the boxed predicate used by [`Scope::when`].
enum ScopeKind<M: Machine> {
    Always,
    On(&'static [M::Event]),
    Entering(&'static [M::State]),
    Leaving(&'static [M::State]),
    When(Box<Predicate<M>>),
}

impl<M: Machine> Scope<M> {
    /// Every transition.
    pub fn always() -> Self {
        Self(ScopeKind::Always)
    }

    /// Only these events.
    pub fn on(events: &'static [M::Event]) -> Self {
        Self(ScopeKind::On(events))
    }

    /// Reaches one of these states from a different one (self-loops excluded).
    pub fn entering(states: &'static [M::State]) -> Self {
        Self(ScopeKind::Entering(states))
    }

    /// Leaves one of these states towards a different one (self-loops excluded).
    pub fn leaving(states: &'static [M::State]) -> Self {
        Self(ScopeKind::Leaving(states))
    }

    /// Any condition over the [`Step`]: the origin state including self-loops
    /// (`step.from == State::Review`), from-any mode, input data, combinations...
    pub fn when(predicate: impl Fn(&Step<'_, M>) -> bool + Send + Sync + 'static) -> Self {
        Self(ScopeKind::When(Box::new(predicate)))
    }

    pub(crate) fn matches(&self, step: &Step<'_, M>) -> bool {
        match &self.0 {
            ScopeKind::Always => true,
            ScopeKind::On(events) => events.contains(&step.event),
            ScopeKind::Entering(states) => !step.is_self_loop() && states.contains(&step.to),
            ScopeKind::Leaving(states) => !step.is_self_loop() && states.contains(&step.from),
            ScopeKind::When(predicate) => predicate(step),
        }
    }
}

impl<M: Machine> fmt::Debug for Scope<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ScopeKind::Always => f.write_str("Scope::Always"),
            ScopeKind::On(events) => f.debug_tuple("Scope::On").field(events).finish(),
            ScopeKind::Entering(states) => f.debug_tuple("Scope::Entering").field(states).finish(),
            ScopeKind::Leaving(states) => f.debug_tuple("Scope::Leaving").field(states).finish(),
            ScopeKind::When(_) => f.write_str("Scope::When(..)"),
        }
    }
}

/// A named rule built from a closure.
///
/// Constructed with [`guard`], [`mutate`], [`verify`], or [`emit`].
#[must_use]
pub struct NamedRule<M: Machine> {
    name: &'static str,
    kind: RuleKind<M>,
}

/// The single behavior a [`NamedRule`] can have.
enum RuleKind<M: Machine> {
    Check(Box<CheckFn<M>>),
    Apply(Box<ApplyFn<M>>),
    Verify(Box<VerifyFn<M>>),
    Effects(Box<EffectsFn<M>>),
}

impl<M: Machine> Rule<M> for NamedRule<M> {
    fn check(&self, step: &Step<'_, M>) -> Result<(), M::Error> {
        match &self.kind {
            RuleKind::Check(check) => check(step),
            _ => Ok(()),
        }
    }

    fn apply(&self, step: &Step<'_, M>, entity: &mut M::Entity) {
        if let RuleKind::Apply(apply) = &self.kind {
            apply(step, entity);
        }
    }

    fn verify(&self, step: &Step<'_, M>, next: &M::Entity) -> Result<(), M::Error> {
        match &self.kind {
            RuleKind::Verify(verify) => verify(step, next),
            _ => Ok(()),
        }
    }

    fn effects(&self, step: &Step<'_, M>, next: &M::Entity) -> Vec<M::Effect> {
        match &self.kind {
            RuleKind::Effects(effects) => effects(step, next),
            _ => Vec::new(),
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

impl<M: Machine> fmt::Debug for NamedRule<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedRule")
            .field("name", &self.name)
            .finish()
    }
}

/// Creates a named guard: a precondition without writing an `impl Rule`.
///
/// ```
/// # use fsm_forge::{guard, Machine, Rule, Step};
/// # struct M;
/// # impl Machine for M {
/// #     type State = u8;
/// #     type Event = u8;
/// #     type Entity = u8;
/// #     type Input = ();
/// #     type Effect = ();
/// #     type Error = &'static str;
/// #     fn state(entity: &u8) -> u8 { *entity }
/// #     fn set_state(entity: &mut u8, state: u8) { *entity = state; }
/// # }
/// let rule = guard::<M>("positive", |step| {
///     if *step.entity > 0 { Ok(()) } else { Err("must be positive") }
/// });
/// assert_eq!(rule.name(), "positive");
/// ```
pub fn guard<M: Machine>(
    name: &'static str,
    check: impl Fn(&Step<'_, M>) -> Result<(), M::Error> + Send + Sync + 'static,
) -> NamedRule<M> {
    NamedRule {
        name,
        kind: RuleKind::Check(Box::new(check)),
    }
}

/// Creates a named change: a mutation without writing an `impl Rule`.
pub fn mutate<M: Machine>(
    name: &'static str,
    apply: impl Fn(&Step<'_, M>, &mut M::Entity) + Send + Sync + 'static,
) -> NamedRule<M> {
    NamedRule {
        name,
        kind: RuleKind::Apply(Box::new(apply)),
    }
}

/// Creates a named post-condition: it sees the modified entity and the destination state.
pub fn verify<M: Machine>(
    name: &'static str,
    check: impl Fn(&Step<'_, M>, &M::Entity) -> Result<(), M::Error> + Send + Sync + 'static,
) -> NamedRule<M> {
    NamedRule {
        name,
        kind: RuleKind::Verify(Box::new(check)),
    }
}

/// Creates a rule that emits a single effect.
///
/// Requires `M::Effect: Clone + Send + Sync + 'static`: rules are shared across threads
/// and the engine asks for effects by reference.
pub fn emit<M: Machine>(name: &'static str, effect: M::Effect) -> NamedRule<M>
where
    M::Effect: Clone + Send + Sync + 'static,
{
    NamedRule {
        name,
        kind: RuleKind::Effects(Box::new(move |_step, _next| vec![effect.clone()])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct M;

    impl Machine for M {
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
    }

    struct Plain;

    impl Rule<M> for Plain {}

    /// A rule generic over a type with a module path, like `Audit<app::Invoice>`.
    struct Audit<T>(T);

    impl<T: Send + Sync + 'static> Rule<M> for Audit<T> {}

    #[test]
    fn default_name_drops_the_module_path() {
        assert_eq!(Plain.name(), "Plain");
    }

    #[test]
    fn default_name_drops_generic_arguments_and_their_paths() {
        // `type_name` is `...::Audit<alloc::string::String>`: cutting at the last `::`
        // first would give `String>`.
        assert_eq!(Audit(String::new()).name(), "Audit");
        assert_eq!(Audit(vec![Plain]).name(), "Audit");
    }

    #[test]
    fn short_type_name_handles_nested_generics() {
        assert_eq!(
            short_type_name::<std::collections::HashMap<String, Vec<u8>>>(),
            "HashMap"
        );
    }
}

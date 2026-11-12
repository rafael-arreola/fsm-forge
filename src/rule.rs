//! Shared transition logic: [`Rule`], [`Scope`], and [`Step`].
//!
//! A [`Rule`] is logic without topology, registered with a [`Scope`] so it runs on every
//! transition the scope selects. An [`Edge`](crate::Edge) declares the same optional phases
//! (`check`, `apply`, `verify`, `effects`) for one fixed transition.
//!
//! [`Step`] is the read-only context both receive. The engine runs the edge first, then
//! every matching scoped rule in registration order, merging their phases.
//!
//! Use a scoped rule when the same logic applies to many transitions:
//!
//! - an effect that must run on every transition, or whenever entering a state;
//! - a precondition shared by several events or target states;
//! - an audit field updated on every accepted transition.
//!
//! For inline closures you do not need to implement [`Rule`]: use
//! [`EngineBuilder::check`](crate::EngineBuilder::check),
//! [`EngineBuilder::apply`](crate::EngineBuilder::apply),
//! [`EngineBuilder::verify`](crate::EngineBuilder::verify),
//! [`EngineBuilder::effects`](crate::EngineBuilder::effects), or
//! [`EngineBuilder::emit`](crate::EngineBuilder::emit).

use std::fmt;

use crate::machine::Machine;

/// The logic of a global rule registered with a [`Scope`]. Every method is optional.
///
/// An [`Edge`](crate::Edge) defines the same four phases for a single transition; `Rule`
/// defines them once for every transition its [`Scope`] selects. Both run through the same
/// pipeline: the edge first, then the matching scoped rules in registration order.
///
/// Implementations must be observationally pure across `check`, `apply`, and `verify` so
/// [`Engine::diagnose`](crate::Engine::diagnose) agrees with
/// [`Engine::execute`](crate::Engine::execute).
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

/// The type name without module path or trailing generic arguments:
/// `app::Outer<u8>::Audit<app::Invoice>` becomes `Audit`.
pub(crate) fn short_type_name<T: ?Sized>() -> &'static str {
    let full = std::any::type_name::<T>();
    if full.starts_with(['(', '[', '&', '*', '!'])
        || full.starts_with("dyn ")
        || full.starts_with("fn(")
        || full.starts_with("unsafe ")
        || full.starts_with("extern ")
        || full.starts_with("for<")
    {
        return full;
    }

    let bytes = full.as_bytes();
    let mut end = bytes.len();
    if bytes.last() == Some(&b'>') {
        let mut depth = 0usize;
        while end > 0 {
            end -= 1;
            match bytes[end] {
                b'>' if end == 0 || bytes[end - 1] != b'-' => depth += 1,
                b'<' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    let name = full[..end].rsplit("::").next().unwrap_or("");
    if name.is_empty() { full } else { name }
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

    /// `true` when the transition runs through `Engine::execute_from_any` or
    /// `Engine::diagnose_from_any`.
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
            .field("event", &self.event)
            .field("from", &self.from)
            .field("to", &self.to)
            .field("input", &self.input)
            .field("from_any", &self.from_any)
            .finish()
    }
}

type Predicate<M> = dyn Fn(&Step<'_, M>) -> bool + Send + Sync;

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

// Single-phase closure adapters registered by `EngineBuilder::{check, apply, verify, effects}`.
pub(crate) struct CheckRule<F>(pub(crate) &'static str, pub(crate) F);
pub(crate) struct ApplyRule<F>(pub(crate) &'static str, pub(crate) F);
pub(crate) struct VerifyRule<F>(pub(crate) &'static str, pub(crate) F);
pub(crate) struct EffectsRule<F>(pub(crate) &'static str, pub(crate) F);

impl<M, F> Rule<M> for CheckRule<F>
where
    M: Machine,
    F: Fn(&Step<'_, M>) -> Result<(), M::Error> + Send + Sync + 'static,
{
    fn check(&self, step: &Step<'_, M>) -> Result<(), M::Error> {
        (self.1)(step)
    }

    fn name(&self) -> &'static str {
        self.0
    }
}

impl<M, F> Rule<M> for ApplyRule<F>
where
    M: Machine,
    F: Fn(&Step<'_, M>, &mut M::Entity) + Send + Sync + 'static,
{
    fn apply(&self, step: &Step<'_, M>, entity: &mut M::Entity) {
        (self.1)(step, entity);
    }

    fn name(&self) -> &'static str {
        self.0
    }
}

impl<M, F> Rule<M> for VerifyRule<F>
where
    M: Machine,
    F: Fn(&Step<'_, M>, &M::Entity) -> Result<(), M::Error> + Send + Sync + 'static,
{
    fn verify(&self, step: &Step<'_, M>, next: &M::Entity) -> Result<(), M::Error> {
        (self.1)(step, next)
    }

    fn name(&self) -> &'static str {
        self.0
    }
}

impl<M, F> Rule<M> for EffectsRule<F>
where
    M: Machine,
    F: Fn(&Step<'_, M>, &M::Entity) -> Vec<M::Effect> + Send + Sync + 'static,
{
    fn effects(&self, step: &Step<'_, M>, next: &M::Entity) -> Vec<M::Effect> {
        (self.1)(step, next)
    }

    fn name(&self) -> &'static str {
        self.0
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

    struct Audit<T>(T);

    impl<T: Send + Sync + 'static> Rule<M> for Audit<T> {}

    struct Outer<T>(std::marker::PhantomData<T>);

    impl<T> Outer<T> {
        fn inner_name() -> &'static str {
            struct InnerRule;
            short_type_name::<InnerRule>()
        }
    }

    #[test]
    fn default_name_drops_the_module_path() {
        assert_eq!(Plain.name(), "Plain");
    }

    #[test]
    fn default_name_drops_generic_arguments_and_their_paths() {
        assert_eq!(Audit(String::new()).name(), "Audit");
        assert_eq!(Audit(vec![Plain]).name(), "Audit");
    }

    #[test]
    fn short_type_name_handles_nested_generics_and_fn_arrows() {
        assert_eq!(
            short_type_name::<std::collections::HashMap<String, Vec<u8>>>(),
            "HashMap"
        );
        assert_eq!(short_type_name::<Audit<fn(u8) -> bool>>(), "Audit");
        assert_eq!(
            Outer::<std::collections::HashMap<String, u8>>::inner_name(),
            "InnerRule"
        );
    }

    #[test]
    fn short_type_name_leaves_non_nominal_types_intact() {
        assert!(short_type_name::<(std::cmp::Ordering, u8)>().starts_with('('));
        assert!(short_type_name::<[std::cmp::Ordering; 2]>().starts_with('['));
        assert!(short_type_name::<dyn std::fmt::Display + Send + Sync>().starts_with("dyn "));
        assert!(short_type_name::<fn(std::cmp::Ordering) -> bool>().starts_with("fn("));
    }
}

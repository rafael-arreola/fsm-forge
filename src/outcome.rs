use std::fmt;

use crate::error::Rejection;
use crate::machine::Machine;

/// An effect together with the rule that produced it.
pub struct AppliedEffect<M: Machine> {
    /// Name of the rule that generated it.
    pub rule: &'static str,

    /// The effect itself.
    pub effect: M::Effect,
}

impl<M: Machine> fmt::Debug for AppliedEffect<M>
where
    M::Effect: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppliedEffect")
            .field("rule", &self.rule)
            .field("effect", &self.effect)
            .finish()
    }
}

/// The transition the engine selected and the rules that took part in it.
///
/// It is what `execute` and `diagnose` have in common: [`Outcome::transition`] on an
/// execution, and the `Ok` of a [`Verdict`] on a diagnosis.
pub struct Transition<M: Machine> {
    /// The event.
    pub event: M::Event,

    /// Origin state.
    pub from: M::State,

    /// Destination state.
    pub to: M::State,

    /// Rules that took part, in execution order: the edge first, then the scoped rules.
    pub rules: Vec<&'static str>,

    /// `true` if it ran (or was diagnosed) ignoring the origin state, through
    /// `execute_from_any` or `diagnose_from_any`.
    pub from_any: bool,
}

impl<M: Machine> Transition<M> {
    /// `true` if origin and destination are the same state.
    pub fn is_self_loop(&self) -> bool {
        self.from == self.to
    }
}

impl<M: Machine> fmt::Debug for Transition<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transition")
            .field("event", &M::event_name(self.event))
            .field("from", &M::state_name(self.from))
            .field("to", &M::state_name(self.to))
            .field("rules", &self.rules)
            .field("from_any", &self.from_any)
            .finish()
    }
}

/// Result of an accepted transition.
pub struct Outcome<M: Machine> {
    /// What ran: event, origin, destination, rules, and mode.
    pub transition: Transition<M>,

    /// Already-modified entity, ready to persist.
    pub entity: M::Entity,

    /// Effects to run after persisting, in rule order.
    pub effects: Vec<AppliedEffect<M>>,
}

impl<M: Machine> Outcome<M> {
    /// The effects without the rule that produced them.
    pub fn effect_values(&self) -> impl Iterator<Item = &M::Effect> {
        self.effects.iter().map(|applied| &applied.effect)
    }

    /// The effects produced by a specific rule.
    pub fn effects_of(&self, rule: &'static str) -> impl Iterator<Item = &M::Effect> {
        self.effects
            .iter()
            .filter(move |applied| applied.rule == rule)
            .map(|applied| &applied.effect)
    }
}

impl<M: Machine> fmt::Debug for Outcome<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Outcome")
            .field("transition", &self.transition)
            .field("effects", &self.effects.len())
            .finish()
    }
}

/// Result of diagnosing a transition: the [`Transition`] that `execute` would run, or
/// the [`Rejection`] it would return. Without entity or effects.
pub type Verdict<M> = Result<Transition<M>, Rejection<M>>;

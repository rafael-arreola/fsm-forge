use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::edge::{CompiledEdge, Edge};
use crate::engine::Engine;
use crate::error::{BuildError, BuildErrors};
use crate::machine::Machine;
use crate::rule::{ApplyRule, CheckRule, EffectsRule, Rule, Scope, Step, VerifyRule};

/// A machine declaration. `build()` validates it and returns the [`Engine`].
#[must_use]
pub struct EngineBuilder<M: Machine> {
    edges: Vec<CompiledEdge<M>>,
    roots: Vec<M::State>,
    rules: Vec<(Scope<M>, Box<dyn Rule<M>>)>,
    required_events: Vec<M::Event>,
    required_states: Vec<M::State>,
}

impl<M: Machine> Default for EngineBuilder<M> {
    fn default() -> Self {
        Self {
            edges: Vec::new(),
            roots: Vec::new(),
            rules: Vec::new(),
            required_events: Vec::new(),
            required_states: Vec::new(),
        }
    }
}

impl<M: Machine> fmt::Debug for EngineBuilder<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rules: Vec<&'static str> = self.rules.iter().map(|(_, rule)| rule.name()).collect();
        f.debug_struct("EngineBuilder")
            .field("edges", &self.edges.len())
            .field("roots", &self.roots)
            .field("rules", &rules)
            .finish_non_exhaustive()
    }
}

/// Tracks the fallback edge `execute_from_any` uses when no edge starts at the current state.
enum FromAnyFallback {
    Single(usize),
    Multiple,
    Originless(usize),
}

impl<M: Machine> EngineBuilder<M> {
    /// Registers an edge declared with [`Edge`].
    pub fn edge<E: Edge<M>>(mut self, edge: E) -> Self {
        self.edges.push(CompiledEdge::new(edge));
        self
    }

    /// Global rule with a scope: its logic runs on every transition the scope selects.
    ///
    /// Only [`Rule`] implementations are accepted here; passing an [`Edge`] is rejected at
    /// compile time:
    ///
    /// ```compile_fail
    /// use fsm_forge::{Edge, Engine, Machine, Scope};
    ///
    /// struct M;
    /// impl Machine for M {
    ///     type State = u8;
    ///     type Event = u8;
    ///     type Entity = u8;
    ///     type Input = ();
    ///     type Effect = ();
    ///     type Error = &'static str;
    ///     fn state(e: &u8) -> u8 { *e }
    ///     fn set_state(e: &mut u8, s: u8) { *e = s; }
    /// }
    ///
    /// struct Step0To1;
    /// impl Edge<M> for Step0To1 {
    ///     const EVENT: u8 = 1;
    ///     const FROM: &'static [u8] = &[0];
    ///     const TO: u8 = 1;
    /// }
    ///
    /// let _ = Engine::<M>::builder().rule(Scope::always(), Step0To1);
    /// ```
    pub fn rule(mut self, scope: Scope<M>, rule: impl Rule<M>) -> Self {
        self.rules.push((scope, Box::new(rule)));
        self
    }

    /// Registers a named precondition (`check`) on every transition selected by `scope`.
    pub fn check(
        self,
        scope: Scope<M>,
        name: &'static str,
        f: impl Fn(&Step<'_, M>) -> Result<(), M::Error> + Send + Sync + 'static,
    ) -> Self {
        self.rule(scope, CheckRule(name, f))
    }

    /// Registers a named mutation (`apply`) on every transition selected by `scope`.
    pub fn apply(
        self,
        scope: Scope<M>,
        name: &'static str,
        f: impl Fn(&Step<'_, M>, &mut M::Entity) + Send + Sync + 'static,
    ) -> Self {
        self.rule(scope, ApplyRule(name, f))
    }

    /// Registers a named post-condition (`verify`) over the modified entity and new state.
    pub fn verify(
        self,
        scope: Scope<M>,
        name: &'static str,
        f: impl Fn(&Step<'_, M>, &M::Entity) -> Result<(), M::Error> + Send + Sync + 'static,
    ) -> Self {
        self.rule(scope, VerifyRule(name, f))
    }

    /// Registers a named effect producer (`effects`) that can read the step and modified entity.
    pub fn effects(
        self,
        scope: Scope<M>,
        name: &'static str,
        f: impl Fn(&Step<'_, M>, &M::Entity) -> Vec<M::Effect> + Send + Sync + 'static,
    ) -> Self {
        self.rule(scope, EffectsRule(name, f))
    }

    /// Registers a named rule that emits a single cloned effect on every matching transition.
    pub fn emit(self, scope: Scope<M>, name: &'static str, effect: M::Effect) -> Self
    where
        M::Effect: Clone + Send + Sync + 'static,
    {
        self.effects(scope, name, move |_step, _next| vec![effect.clone()])
    }

    /// Declares the primary entry state returned by [`Engine::initial`], placing it first in
    /// [`Engine::roots`].
    ///
    /// With roots declared, `build()` verifies that every state in the topology is
    /// reachable from at least one of them.
    pub fn initial(mut self, state: M::State) -> Self {
        self.roots.retain(|&existing| existing != state);
        self.roots.insert(0, state);
        self
    }

    /// Declares several entry states, for example when the entity can be born in more
    /// than one state.
    pub fn roots(mut self, states: impl IntoIterator<Item = M::State>) -> Self {
        for state in states {
            if !self.roots.contains(&state) {
                self.roots.push(state);
            }
        }
        self
    }

    /// `build()` fails if any of these events has no edge
    /// (e.g. `Event::iter()` with strum).
    pub fn require_events(mut self, events: impl IntoIterator<Item = M::Event>) -> Self {
        self.required_events.extend(events);
        self
    }

    /// `build()` fails if any of these states does not appear in any edge and is
    /// not a declared root, e.g. `State::iter()` with strum.
    pub fn require_states(mut self, states: impl IntoIterator<Item = M::State>) -> Self {
        self.required_states.extend(states);
        self
    }

    /// Validates the declaration and returns the engine, or ALL the errors found.
    pub fn build(self) -> Result<Engine<M>, BuildErrors<M>> {
        let mut errors = Vec::new();
        let mut push_error = |error: BuildError<M>| {
            if !errors.contains(&error) {
                errors.push(error);
            }
        };

        let mut by_origin = HashMap::new();
        let mut by_state: HashMap<M::State, Vec<M::Event>> = HashMap::new();
        let mut fallbacks: HashMap<M::Event, FromAnyFallback> = HashMap::new();

        for (index, edge) in self.edges.iter().enumerate() {
            match fallbacks.entry(edge.event) {
                Entry::Vacant(slot) => {
                    let fb = if edge.from.is_empty() {
                        FromAnyFallback::Originless(index)
                    } else {
                        FromAnyFallback::Single(index)
                    };
                    slot.insert(fb);
                }
                Entry::Occupied(mut slot) => {
                    if edge.from.is_empty() {
                        if matches!(slot.get(), FromAnyFallback::Originless(_)) {
                            push_error(BuildError::AmbiguousOriginless(edge.event));
                        } else {
                            slot.insert(FromAnyFallback::Originless(index));
                        }
                    } else if matches!(slot.get(), FromAnyFallback::Single(_)) {
                        slot.insert(FromAnyFallback::Multiple);
                    }
                }
            }

            for from in &edge.from {
                match by_origin.entry((edge.event, *from)) {
                    Entry::Vacant(slot) => {
                        slot.insert(index);
                        by_state.entry(*from).or_default().push(edge.event);
                    }
                    Entry::Occupied(_) => {
                        push_error(BuildError::Conflict {
                            event: edge.event,
                            from: *from,
                        });
                    }
                }
            }
        }

        for event in &self.required_events {
            if !fallbacks.contains_key(event) {
                push_error(BuildError::MissingEvent(*event));
            }
        }

        let mut mentioned: HashSet<M::State> = self.roots.iter().copied().collect();
        for edge in &self.edges {
            mentioned.extend(edge.from.iter().copied());
            mentioned.insert(edge.to);
        }
        for state in &self.required_states {
            if !mentioned.contains(state) {
                push_error(BuildError::MissingState(*state));
            }
        }

        // Reachability: only when roots are declared, because without them
        // "reachable" has no definition.
        if !self.roots.is_empty() {
            let mut reachable: HashSet<M::State> = self.roots.iter().copied().collect();
            loop {
                let mut changed = false;
                for edge in &self.edges {
                    let reached = if edge.from.is_empty() {
                        reachable
                            .iter()
                            .any(|state| !by_origin.contains_key(&(edge.event, *state)))
                    } else {
                        edge.from.iter().any(|from| reachable.contains(from))
                    };
                    if reached {
                        changed |= reachable.insert(edge.to);
                    }
                }
                if !changed {
                    break;
                }
            }

            for edge in &self.edges {
                if edge.from.is_empty()
                    && reachable
                        .iter()
                        .all(|state| by_origin.contains_key(&(edge.event, *state)))
                {
                    push_error(BuildError::ShadowedOriginless(edge.event));
                }
                for state in edge.from.iter().chain(std::iter::once(&edge.to)) {
                    if !reachable.contains(state) {
                        push_error(BuildError::UnreachableState(*state));
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(BuildErrors::new(errors));
        }

        let by_event = fallbacks
            .into_iter()
            .map(|(event, fb)| {
                let target = match fb {
                    FromAnyFallback::Single(i) | FromAnyFallback::Originless(i) => Some(i),
                    FromAnyFallback::Multiple => None,
                };
                (event, target)
            })
            .collect();

        Ok(Engine {
            edges: self.edges,
            roots: self.roots,
            rules: self.rules,
            by_origin,
            by_event,
            by_state,
        })
    }
}

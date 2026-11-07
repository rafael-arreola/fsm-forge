use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::edge::{CompiledEdge, Edge};
use crate::engine::{Engine, EventIndex};
use crate::error::{BuildError, BuildErrors};
use crate::machine::Machine;
use crate::rule::{Rule, Scope};

/// A machine declaration. `build()` validates it and returns the [`Engine`].
#[must_use]
pub struct EngineBuilder<M: Machine> {
    edges: Vec<CompiledEdge<M>>,
    roots: Vec<M::State>,
    rules: Vec<(Scope<M>, Box<dyn Rule<M>>)>,
    required: Vec<M::Event>,
    required_states: Vec<M::State>,
}

impl<M: Machine> Default for EngineBuilder<M> {
    fn default() -> Self {
        Self {
            edges: Vec::new(),
            roots: Vec::new(),
            rules: Vec::new(),
            required: Vec::new(),
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
            .finish()
    }
}

impl<M: Machine> EngineBuilder<M> {
    /// Registers an edge declared with [`Edge`].
    pub fn edge<E: Edge<M>>(mut self, edge: E) -> Self {
        self.edges.push(CompiledEdge::new(edge));
        self
    }

    /// Global rule with a scope: its logic runs on every transition the scope selects.
    ///
    /// # Do not register an edge here
    ///
    /// An [`Edge`] is also a [`Rule`], so the compiler accepts it, and neither the
    /// compiler nor `build()` reports it. Its logic would run on every transition the
    /// scope selects, not only on its own:
    ///
    /// ```text
    /// .edge(SolutionConfirmed)                    // correct: only Resolved -> Closed
    /// .rule(Scope::always(), SolutionConfirmed)   // wrong: its check, apply, and effects
    ///                                             // run on every transition
    /// ```
    ///
    /// Register edges with [`EngineBuilder::edge`].
    pub fn rule(mut self, scope: Scope<M>, rule: impl Rule<M>) -> Self {
        self.rules.push((scope, Box::new(rule)));
        self
    }

    /// Declares the entry state (the "state 0"). Calling it again accumulates roots.
    ///
    /// With roots declared, `build()` verifies that every state in the topology is
    /// reachable from at least one of them.
    pub fn initial(mut self, state: M::State) -> Self {
        self.push_root(state);
        self
    }

    /// Declares several entry states, for example when the entity can be born in more
    /// than one state.
    pub fn roots(mut self, states: impl IntoIterator<Item = M::State>) -> Self {
        for state in states {
            self.push_root(state);
        }
        self
    }

    /// `build()` fails if any of these events has no edge
    /// (e.g. `Event::iter()` with strum).
    pub fn require_events(mut self, events: impl IntoIterator<Item = M::Event>) -> Self {
        self.required.extend(events);
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

        let mut by_origin = HashMap::new();
        let mut by_event: HashMap<M::Event, EventIndex> = HashMap::new();
        // Duplicates already reported: each one is reported once, however many edges
        // repeat it.
        let mut conflicts = HashSet::new();
        let mut ambiguous = HashSet::new();

        for (index, edge) in self.edges.iter().enumerate() {
            let event_index = by_event.entry(edge.event).or_default();
            event_index.edges.push(index);

            // The first edge without origin of an event is the one `execute_from_any`
            // uses; any later one makes the choice ambiguous.
            if edge.from.is_empty() {
                if event_index.originless.is_none() {
                    event_index.originless = Some(index);
                } else if ambiguous.insert(edge.event) {
                    errors.push(BuildError::AmbiguousOriginless { event: edge.event });
                }
            }

            for from in &edge.from {
                match by_origin.entry((edge.event, *from)) {
                    Entry::Vacant(slot) => {
                        slot.insert(index);
                    }
                    Entry::Occupied(_) => {
                        if conflicts.insert((edge.event, *from)) {
                            errors.push(BuildError::Conflict {
                                event: edge.event,
                                from: *from,
                            });
                        }
                    }
                }
            }
        }

        let mut missing = HashSet::new();
        for event in &self.required {
            if !by_event.contains_key(event) && missing.insert(*event) {
                errors.push(BuildError::MissingEvent(*event));
            }
        }

        let mut mentioned = HashSet::new();
        mentioned.extend(self.roots.iter().copied());
        for edge in &self.edges {
            mentioned.extend(edge.from.iter().copied());
            mentioned.insert(edge.to);
        }
        let mut missing_states = HashSet::new();
        for state in &self.required_states {
            if !mentioned.contains(state) && missing_states.insert(*state) {
                errors.push(BuildError::MissingState(*state));
            }
        }

        // Reachability: only when roots are declared, because without them
        // "reachable" has no definition.
        if !self.roots.is_empty() {
            let mut reachable: HashSet<M::State> = self.roots.iter().copied().collect();
            loop {
                let mut changed = false;
                for edge in &self.edges {
                    // An edge without origin is used by `execute_from_any` when the
                    // current state has no edge for that event; if every reachable state
                    // already has one, it is never selected.
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

            let mut reported = HashSet::new();
            for edge in &self.edges {
                for state in edge.from.iter().chain(std::iter::once(&edge.to)) {
                    if !reachable.contains(state) && reported.insert(*state) {
                        errors.push(BuildError::UnreachableState(*state));
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(BuildErrors::new(errors));
        }

        Ok(Engine {
            edges: self.edges,
            roots: self.roots,
            rules: self.rules,
            by_origin,
            by_event,
        })
    }

    fn push_root(&mut self, state: M::State) {
        if !self.roots.contains(&state) {
            self.roots.push(state);
        }
    }
}

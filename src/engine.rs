use std::collections::HashMap;
use std::fmt;

use crate::builder::EngineBuilder;
use crate::edge::CompiledEdge;
use crate::error::{AppliedError, Phase, Rejection};
use crate::machine::Machine;
use crate::outcome::{AppliedEffect, Outcome, Transition, Verdict};
use crate::rule::{Rule, Scope, Step};

/// The validated engine. Immutable and reusable.
///
/// Built only by [`EngineBuilder::build`], which guarantees the invariants the indexes
/// rely on: every index points into `edges`, and each event has a precomputed `from_any`
/// fallback when unambiguous.
#[must_use]
pub struct Engine<M: Machine> {
    pub(crate) edges: Vec<CompiledEdge<M>>,
    pub(crate) roots: Vec<M::State>,
    pub(crate) rules: Vec<(Scope<M>, Box<dyn Rule<M>>)>,
    pub(crate) by_origin: HashMap<(M::Event, M::State), usize>,
    pub(crate) by_event: HashMap<M::Event, Option<usize>>,
    pub(crate) by_state: HashMap<M::State, Vec<M::Event>>,
}

impl<M: Machine> fmt::Debug for Engine<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rules: Vec<&'static str> = self.rules.iter().map(|(_, rule)| rule.name()).collect();
        f.debug_struct("Engine")
            .field("edges", &self.edges.len())
            .field("roots", &self.roots)
            .field("rules", &rules)
            .finish_non_exhaustive()
    }
}

/// Internal result of evaluating a transition, before collecting effects.
struct Evaluated<'a, M: Machine> {
    step: Step<'a, M>,
    next: M::Entity,
    rules: Vec<&'a dyn Rule<M>>,
}

impl<M: Machine> Evaluated<'_, M> {
    fn into_transition(self) -> Transition<M> {
        Transition {
            event: self.step.event,
            from: self.step.from,
            to: self.step.to,
            rules: self.rules.iter().map(|rule| rule.name()).collect(),
            from_any: self.step.from_any,
        }
    }

    fn into_outcome(self) -> Outcome<M> {
        let transition = Transition {
            event: self.step.event,
            from: self.step.from,
            to: self.step.to,
            rules: self.rules.iter().map(|rule| rule.name()).collect(),
            from_any: self.step.from_any,
        };
        let effects = self
            .rules
            .iter()
            .flat_map(|rule| {
                let name = rule.name();
                rule.effects(&self.step, &self.next)
                    .into_iter()
                    .map(move |effect| AppliedEffect { rule: name, effect })
            })
            .collect();
        Outcome {
            transition,
            entity: self.next,
            effects,
        }
    }
}

impl<M: Machine> Engine<M> {
    /// Starts declaring a machine.
    pub fn builder() -> EngineBuilder<M> {
        EngineBuilder::default()
    }

    /// Normal transition: the current state must be an origin of a transition for the event.
    pub fn execute(
        &self,
        entity: &M::Entity,
        event: M::Event,
        input: &M::Input,
    ) -> Result<Outcome<M>, Rejection<M>> {
        self.evaluate(entity, event, input, false)
            .map(Evaluated::into_outcome)
    }

    /// From-any execution: prefers an edge starting at the current state, otherwise selects
    /// the event's originless edge or sole edge. Every rule still runs.
    pub fn execute_from_any(
        &self,
        entity: &M::Entity,
        event: M::Event,
        input: &M::Input,
    ) -> Result<Outcome<M>, Rejection<M>> {
        self.evaluate(entity, event, input, true)
            .map(Evaluated::into_outcome)
    }

    /// Events with a normal transition from the current state, in declaration order.
    ///
    /// It only looks at the topology: rules are evaluated on execution.
    #[must_use]
    pub fn available(&self, entity: &M::Entity) -> &[M::Event] {
        self.by_state
            .get(&M::state(entity))
            .map_or(&[], Vec::as_slice)
    }

    /// The verdict of `execute` without the entity or the effects.
    ///
    /// It runs the same pattern (selection, `check`, `apply`, `verify`), so for pure
    /// `Entity::clone` and rule implementations an `Ok` guarantees that `execute` would
    /// accept it too. It does not call `Edge::effects` or `Rule::effects`.
    pub fn diagnose(&self, entity: &M::Entity, event: M::Event, input: &M::Input) -> Verdict<M> {
        self.evaluate(entity, event, input, false)
            .map(Evaluated::into_transition)
    }

    /// Same as `diagnose`, using from-any edge selection.
    pub fn diagnose_from_any(
        &self,
        entity: &M::Entity,
        event: M::Event,
        input: &M::Input,
    ) -> Verdict<M> {
        self.evaluate(entity, event, input, true)
            .map(Evaluated::into_transition)
    }

    /// `diagnose` for every event in `available`, in the same order.
    ///
    /// The `Input` is the one from the current request: useful for action screens
    /// ("what can I do from here and why not?"). It does not include transitions without
    /// origin; those only run through `execute_from_any`.
    ///
    /// Because checks can read `input`, the verdict is only as reliable as the `Input`
    /// you pass. It does not call `Edge::effects` or `Rule::effects`.
    #[must_use]
    pub fn available_detailed(
        &self,
        entity: &M::Entity,
        input: &M::Input,
    ) -> Vec<(M::Event, Verdict<M>)> {
        self.available(entity)
            .iter()
            .map(|&event| (event, self.diagnose(entity, event, input)))
            .collect()
    }

    /// Destination of `event` from `from` according to the topology.
    ///
    /// It does not evaluate rules or select a from-any edge.
    #[must_use]
    pub fn target(&self, event: M::Event, from: M::State) -> Option<M::State> {
        self.by_origin
            .get(&(event, from))
            .map(|&index| self.edges[index].to)
    }

    /// Declared primary entry state: the first of [`Engine::roots`], if any.
    ///
    /// The engine does not create entities, but this value tells the application layer
    /// in which state a new entity is born.
    #[must_use]
    pub fn initial(&self) -> Option<M::State> {
        self.roots.first().copied()
    }

    /// Every declared entry state, in order.
    #[must_use]
    pub fn roots(&self) -> &[M::State] {
        &self.roots
    }

    fn evaluate<'a>(
        &'a self,
        entity: &'a M::Entity,
        event: M::Event,
        input: &'a M::Input,
        from_any: bool,
    ) -> Result<Evaluated<'a, M>, Rejection<M>> {
        // 1. Edge
        let from = M::state(entity);
        let edge = self.select(event, from, from_any)?;
        let step = Step {
            entity,
            event,
            from,
            to: edge.to,
            input,
            from_any,
        };

        // 2. Rules: the edge logic first, then the matching global rules in
        //    registration order
        let rules: Vec<&dyn Rule<M>> = std::iter::once(&*edge.rule)
            .chain(
                self.rules
                    .iter()
                    .filter(|(scope, _)| scope.matches(&step))
                    .map(|(_, rule)| &**rule),
            )
            .collect();

        // 3. Preconditions
        run_phase(&rules, Phase::Check, |rule| rule.check(&step))?;

        // 4. Changes and new state
        let mut next = entity.clone();
        for rule in &rules {
            rule.apply(&step, &mut next);
        }
        M::set_state(&mut next, edge.to);
        debug_assert_eq!(
            M::state(&next),
            edge.to,
            "Machine::set_state must update the entity state to the target state"
        );

        // 5. Post-conditions over the resulting entity
        run_phase(&rules, Phase::Verify, |rule| rule.verify(&step, &next))?;

        Ok(Evaluated { step, next, rules })
    }

    fn select(
        &self,
        event: M::Event,
        from: M::State,
        from_any: bool,
    ) -> Result<&CompiledEdge<M>, Rejection<M>> {
        let fallback = self
            .by_event
            .get(&event)
            .ok_or(Rejection::UnknownEvent(event))?;

        // An edge from the current state always wins, in both modes.
        if let Some(&position) = self.by_origin.get(&(event, from)) {
            return Ok(&self.edges[position]);
        }
        if !from_any {
            return Err(Rejection::NotAllowed { event, from });
        }

        fallback
            .map(|position| &self.edges[position])
            .ok_or(Rejection::Ambiguous(event))
    }
}

fn run_phase<M: Machine>(
    rules: &[&dyn Rule<M>],
    phase: Phase,
    mut check: impl FnMut(&dyn Rule<M>) -> Result<(), M::Error>,
) -> Result<(), Rejection<M>> {
    let errors: Vec<AppliedError<M>> = rules
        .iter()
        .filter_map(|rule| {
            check(*rule).err().map(|error| AppliedError {
                rule: rule.name(),
                phase,
                error,
            })
        })
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Rejection::Rules(errors))
    }
}

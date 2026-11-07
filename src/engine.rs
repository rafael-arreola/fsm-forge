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
/// rely on: every index points into `edges`, and an event has at most one edge without
/// origin.
#[must_use]
pub struct Engine<M: Machine> {
    pub(crate) edges: Vec<CompiledEdge<M>>,
    pub(crate) roots: Vec<M::State>,
    pub(crate) rules: Vec<(Scope<M>, Box<dyn Rule<M>>)>,
    pub(crate) by_origin: HashMap<(M::Event, M::State), usize>,
    pub(crate) by_event: HashMap<M::Event, EventIndex>,
}

impl<M: Machine> fmt::Debug for Engine<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rules: Vec<&'static str> = self.rules.iter().map(|(_, rule)| rule.name()).collect();
        f.debug_struct("Engine")
            .field("edges", &self.edges.len())
            .field("roots", &self.roots)
            .field("rules", &rules)
            .finish()
    }
}

/// The edges of one event, as positions in `Engine::edges`.
#[derive(Default)]
pub(crate) struct EventIndex {
    /// The edge without origin, if any. `build()` rejects more than one.
    pub(crate) originless: Option<usize>,

    /// Every edge of the event, in declaration order. Never empty.
    pub(crate) edges: Vec<usize>,
}

/// Internal result of evaluating a transition, before collecting effects.
struct Evaluated<'a, M: Machine> {
    step: Step<'a, M>,
    next: M::Entity,
    rules: Vec<&'a dyn Rule<M>>,
}

impl<M: Machine> Evaluated<'_, M> {
    /// The part `execute` and `diagnose` report alike.
    fn transition(&self) -> Transition<M> {
        Transition {
            event: self.step.event,
            from: self.step.from,
            to: self.step.to,
            rules: self.rules.iter().map(|rule| rule.name()).collect(),
            from_any: self.step.from_any,
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
        let evaluated = self.evaluate(entity, event, input, false)?;
        Ok(self.finish(evaluated))
    }

    /// From-any execution: it ignores the origin state. Rules still run.
    pub fn execute_from_any(
        &self,
        entity: &M::Entity,
        event: M::Event,
        input: &M::Input,
    ) -> Result<Outcome<M>, Rejection<M>> {
        let evaluated = self.evaluate(entity, event, input, true)?;
        Ok(self.finish(evaluated))
    }

    /// Events with a normal transition from the current state, in declaration order.
    ///
    /// It only looks at the topology: rules are evaluated on execution.
    #[must_use]
    pub fn available(&self, entity: &M::Entity) -> Vec<M::Event> {
        let state = M::state(entity);
        self.edges
            .iter()
            .filter(|edge| edge.from.contains(&state))
            .map(|edge| edge.event)
            .collect()
    }

    /// The verdict of `execute` without the entity or the effects.
    ///
    /// It runs the same pattern (selection, `check`, `apply`, `verify`), so an `Ok`
    /// guarantees that `execute` would accept it too. It does not call `Rule::effects`.
    pub fn diagnose(&self, entity: &M::Entity, event: M::Event, input: &M::Input) -> Verdict<M> {
        self.diagnose_run(entity, event, input, false)
    }

    /// Same as `diagnose`, ignoring the origin state.
    pub fn diagnose_from_any(
        &self,
        entity: &M::Entity,
        event: M::Event,
        input: &M::Input,
    ) -> Verdict<M> {
        self.diagnose_run(entity, event, input, true)
    }

    /// `diagnose` for every event in `available`, in the same order.
    ///
    /// The `Input` is the one from the current request: useful for action screens
    /// ("what can I do from here and why not?"). It does not include transitions without
    /// origin; those only run through `execute_from_any`.
    ///
    /// Because guards can read `input`, the verdict is only as reliable as the `Input`
    /// you pass. It does not call `Rule::effects`.
    #[must_use]
    pub fn available_detailed(
        &self,
        entity: &M::Entity,
        input: &M::Input,
    ) -> Vec<(M::Event, Verdict<M>)> {
        self.available(entity)
            .into_iter()
            .map(|event| (event, self.diagnose(entity, event, input)))
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

    /// Declared entry state: the first of [`Engine::roots`], if any.
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

    fn diagnose_run(
        &self,
        entity: &M::Entity,
        event: M::Event,
        input: &M::Input,
        from_any: bool,
    ) -> Verdict<M> {
        self.evaluate(entity, event, input, from_any)
            .map(|evaluated| evaluated.transition())
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
        let errors: Vec<AppliedError<M>> = rules
            .iter()
            .filter_map(|rule| {
                rule.check(&step).err().map(|error| AppliedError {
                    rule: rule.name(),
                    phase: Phase::Check,
                    error,
                })
            })
            .collect();
        if !errors.is_empty() {
            return Err(Rejection::Rules(errors));
        }

        // 4. Changes and new state
        let mut next = entity.clone();
        for rule in &rules {
            rule.apply(&step, &mut next);
        }
        M::set_state(&mut next, edge.to);

        // 5. Post-conditions over the resulting entity
        let errors: Vec<AppliedError<M>> = rules
            .iter()
            .filter_map(|rule| {
                rule.verify(&step, &next).err().map(|error| AppliedError {
                    rule: rule.name(),
                    phase: Phase::Verify,
                    error,
                })
            })
            .collect();
        if !errors.is_empty() {
            return Err(Rejection::Rules(errors));
        }

        Ok(Evaluated { step, next, rules })
    }

    /// Collects the effects and builds the `Outcome`. Only the real execution uses it.
    fn finish(&self, evaluated: Evaluated<'_, M>) -> Outcome<M> {
        let transition = evaluated.transition();
        let Evaluated { step, next, rules } = evaluated;
        let effects: Vec<AppliedEffect<M>> = rules
            .iter()
            .flat_map(|rule| {
                let name = rule.name();
                rule.effects(&step, &next)
                    .into_iter()
                    .map(move |effect| AppliedEffect { rule: name, effect })
            })
            .collect();
        Outcome {
            transition,
            entity: next,
            effects,
        }
    }

    fn select(
        &self,
        event: M::Event,
        from: M::State,
        from_any: bool,
    ) -> Result<&CompiledEdge<M>, Rejection<M>> {
        let index = self
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

        // From-any: the edge without origin means "from any state".
        if let Some(position) = index.originless {
            return Ok(&self.edges[position]);
        }

        // Without one, only a single edge for the event is unambiguous.
        match index.edges.as_slice() {
            [only] => Ok(&self.edges[*only]),
            _ => Err(Rejection::Ambiguous(event)),
        }
    }
}

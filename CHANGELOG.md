# Changelog

All notable versions of this project are documented here.
The format follows Keep a Changelog and Semantic Versioning.

## [Unreleased]

First public release.

### Added

- `Machine` trait as the domain contract: the state, event, entity, input, effect, and
  error types, plus how to read and write the state of an entity.
- `Edge` trait: one type per transition, with its topology as the `EVENT`, `FROM`, and
  `TO` constants. Its logic is the `Rule` implementation of the same type; an edge without
  logic implements `Rule` with an empty block. The type may carry configuration in its
  fields; the engine uses the instance passed to `.edge(...)`.
- `Rule` trait with `check`, `apply`, `verify`, `effects`, and `name`, all optional. The
  same trait describes the logic of an edge and of a global rule.
- `Scope` to register global rules: `always`, `on`, `entering`, `leaving`, and `when`.
- Helpers `guard`, `mutate`, `verify`, and `emit` to build global rules from closures.
- `Step` as the execution context: the entity before the change, the event, the origin
  and destination states, the input, and the from-any mode.
- `EngineBuilder` with `edge`, `rule`, `initial`, `roots`, `require_events`, and
  `require_states`. `build()` reports every error together: conflicts, missing events and
  states, ambiguous edges without origin, and states unreachable from the roots.
- `Engine` with `execute`, `execute_from_any`, `diagnose`, `diagnose_from_any`,
  `available`, `available_detailed`, `target`, `initial`, `roots`, and `to_mermaid`.
- `Outcome` with the `Transition` that ran, the resulting entity, and the effects
  attributed to their rule; helpers `effect_values` and `effects_of`.
- `Transition`, shared by executions and diagnoses: event, origin, destination, the rules
  that took part, and the from-any mode.
- `Rejection` with rule errors attributed to their rule and phase (`check` / `verify`),
  accessible through `rules()` and `rule_errors()`.
- Examples `minimal`, `invoice`, `initial`, `conditions`, and `ticket`, ordered by
  difficulty from one edge to a full lifecycle.
- Integration tests for simple topologies, rules, scopes, builder validation, Mermaid,
  and the full ticket lifecycle.

### Behavior

- `execute` and `diagnose` follow the same order: select the edge, run every `check`,
  apply the changes to a copy of the entity, and run every `verify`. Only `execute`
  collects the effects.
- `execute_from_any` ignores the origin state while still running every rule. Edges with
  `FROM: &[]` are only reachable through it.
- Repeated origins in one edge declaration are deduplicated instead of reported as a
  conflict.
- `Transition::rules` names the edge type first, followed by the global rules that
  matched, in registration order.

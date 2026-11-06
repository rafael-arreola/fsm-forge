# Changelog

All notable versions of this project are documented here.
The format follows Keep a Changelog and Semantic Versioning.

## [Unreleased]

### Changed

- **Breaking:** the crate is renamed to `fsm-forge`; import it as `fsm_forge`.
- **Breaking:** `Edge` only declares the topology (`EVENT`, `FROM`, `TO`) and requires
  `Rule`: the logic of an edge is its `Rule` implementation. An edge without logic
  implements `Rule` with an empty block.
- **Breaking:** `Outcome` groups `event`, `from`, `to`, `rules`, and `from_any` in
  `Outcome::transition`, a `Transition`. `is_self_loop()` moves to `Transition`.
- **Breaking:** `Verdict` is `Result<Transition, Rejection>`: `diagnose`,
  `diagnose_from_any`, and `available_detailed` return the same `Transition` that
  `execute` puts in its `Outcome`.
- `build()` reports `AmbiguousOriginless` in declaration order, interleaved with
  `Conflict`, instead of after every conflict.

### Removed

- **Breaking:** `Diagnosis`, replaced by `Transition`.
- **Breaking:** `Step::is_from_any()`; read the `Step::from_any` field.
- **Breaking:** `Scope::within`. It matched every transition whose origin was one of the
  states, self-loops included; `Scope::when(|step| step.from == State::X)` says that
  explicitly.

### Fixed

- The default name of a generic rule whose type argument has a module path
  (`Audit<app::Invoice>`) was the argument with a trailing `>` (`Invoice>`). It is now
  the type name (`Audit`).

## [0.1.0] - 2026-11-04

### Added

- `Machine` trait as the domain contract (state, event, entity, input, effect, error).
- `Edge` trait: one type per transition, with `EVENT`, `FROM`, and `TO` constants and
  `check`, `apply`, `verify`, `effects`, and `name` methods. The type may carry
  configuration in its fields; the engine uses the instance passed to `.edge(...)`.
- `EngineBuilder::edge` registers an `Edge` implementation.
- `Rule` trait with `check`, `apply`, `verify`, and `effects`, all optional, for global
  rules registered with a `Scope`.
- `Step` as the execution context and `Scope` for global rules.
- `Engine` with `execute`, `execute_from_any`, `diagnose`, `diagnose_from_any`,
  `available`, `available_detailed`, `target`, `initial`, and `roots`.
- `EngineBuilder` validation in `build()`: conflicts, missing events and states,
  ambiguous edges without origin, and reachability from roots.
- `Outcome` with the resulting entity, effects attributed to their rule, and the rules
  that participated; helpers `effect_values`, `effects_of`, and `is_self_loop`.
- `Rejection` with rule errors attributed to rule and phase (`check` / `verify`),
  accessible through `rules()` and `rule_errors()`.
- `BuildError::AmbiguousOriginless` for events with more than one edge without origin.
- `to_mermaid` to generate the diagram from the topology.
- Helpers `guard`, `mutate`, `verify`, and `emit` for global rules.
- Examples `minimal`, `invoice`, `initial`, `conditions`, and `ticket`, ordered by
  difficulty from one edge to a full lifecycle.
- Integration tests for simple topologies, rules, scopes, builder validation, Mermaid,
  and the full ticket lifecycle.

### Behavior

- `execute_from_any` ignores the origin state while still running every rule. Edges with
  `FROM: &[]` are only reachable through it.
- Repeated origins in one edge declaration are deduplicated instead of reported as a
  conflict.
- `Outcome::rules` names the edge type once, followed by the global rules that matched,
  in registration order.

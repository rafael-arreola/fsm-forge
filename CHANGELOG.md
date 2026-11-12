# Changelog

All notable versions of this project are documented here.
The format follows Keep a Changelog and Semantic Versioning.

## [Unreleased]

### Added

- Inline scoped-rule methods on `EngineBuilder`: `.check`, `.apply`, `.verify`, `.effects`,
  and `.emit`, with full type inference from the builder's `Machine` type and no double
  boxing.
- `BuildError::ShadowedOriginless` to report edges with `FROM: &[]` that can never be
  selected because every reachable state already defines an explicit edge for that event.
- `Deref<Target = [BuildError<M>]>` on `BuildErrors<M>`, `Display` on `Phase`, and
  `std::error::Error` on `AppliedError<M>`.
- Incremental, numbered example suite (`01_minimal`, `02_phases`, `03_counter`,
  `04_scopes`, `05_diagnostics`, `06_from_any`, `07_topology`, `08_ticket`), introducing
  one library capability per step.

### Changed

- `Edge<M>` is now independent from `Rule<M>` and declares its own optional `check`,
  `apply`, `verify`, `effects`, and `name` methods. Edges no longer require an empty
  `impl Rule<M>` block and cannot be passed to `EngineBuilder::rule` by mistake.
- `Machine::Error` now requires `Debug` in addition to `Display + Send + Sync + 'static`.
- `Engine::available` now returns `&[M::Event]` backed by a build-time state index instead
  of allocating a `Vec<M::Event>`.
- `BuildError::AmbiguousOriginless` is now a tuple variant `AmbiguousOriginless(M::Event)`
  consistent with the other single-field error variants.
- `Outcome<M>` is now marked `#[must_use]`, and `Outcome::effects_of` was removed in favor
  of direct field access on `Outcome::effects`.
- Free helpers `guard`, `mutate`, `verify`, `emit`, `NamedRule`, and `prelude` were
  replaced by the direct `EngineBuilder` phase methods and root exports.
- `Rejection`, `BuildError`, `BuildErrors`, `Step`, and `Transition` now use structural
  `Debug` formatting while keeping human-readable labels in `Display`.

### Fixed

- `Outcome::Debug` now formats `entity` and the full `effects` list so failed `assert_eq!`
  checks show the actual differences.
- `EngineBuilder::initial` now always sets the primary root at index `0` even when called
  after `EngineBuilder::roots`.
- `Rule::name` / `Edge::name` default now preserves inner type names inside generic outer
  types, handles `fn(...) -> ...` type arguments, and leaves non-nominal types intact.
- `Engine::to_mermaid` now handles empty and symbol-only state `Debug` outputs,
  disambiguates all `stateDiagram-v2` reserved keywords, and sanitizes `:`, `;`, and `%%`
  in labels.

## [0.1.0] - 2026-11-07

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

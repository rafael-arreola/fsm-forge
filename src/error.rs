//! The public errors, split by moment: execution ([`Rejection`]) and construction
//! ([`BuildError`] / [`BuildErrors`]).

use std::fmt;
use std::ops::Deref;

use crate::machine::Machine;

/// Phase in which an edge or rule rejected the transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// A precondition failed (`Edge::check` or `Rule::check`).
    Check,

    /// A post-condition failed (`Edge::verify` or `Rule::verify`).
    Verify,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Check => f.write_str("check"),
            Self::Verify => f.write_str("verify"),
        }
    }
}

/// A rule error together with the rule and phase that produced it.
pub struct AppliedError<M: Machine> {
    /// Name of the edge or rule that rejected.
    pub rule: &'static str,

    /// When it rejected.
    pub phase: Phase,

    /// The error itself.
    pub error: M::Error,
}

impl<M: Machine> Clone for AppliedError<M>
where
    M::Error: Clone,
{
    fn clone(&self) -> Self {
        Self {
            rule: self.rule,
            phase: self.phase,
            error: self.error.clone(),
        }
    }
}

impl<M: Machine> PartialEq for AppliedError<M>
where
    M::Error: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.rule == other.rule && self.phase == other.phase && self.error == other.error
    }
}

impl<M: Machine> Eq for AppliedError<M> where M::Error: Eq {}

impl<M: Machine> fmt::Debug for AppliedError<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppliedError")
            .field("rule", &self.rule)
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish()
    }
}

impl<M: Machine> fmt::Display for AppliedError<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]: {}", self.rule, self.phase, self.error)
    }
}

impl<M: Machine> std::error::Error for AppliedError<M> {}

/// Why a transition did not run.
#[non_exhaustive]
pub enum Rejection<M: Machine> {
    /// The event has no declared edge.
    UnknownEvent(M::Event),

    /// No edge for the event starts at the current state.
    NotAllowed {
        /// Requested event.
        event: M::Event,
        /// Current state, which is not an origin of any edge for the event.
        from: M::State,
    },

    /// When overriding: the event has several edges, none starts at the current state,
    /// and none is originless (`FROM: &[]`).
    Ambiguous(M::Event),

    /// One or more rules rejected. Contains ALL errors, with their rule and phase.
    Rules(Vec<AppliedError<M>>),
}

impl<M: Machine> Rejection<M> {
    /// The rule errors without the rule or phase that produced them.
    pub fn rule_errors(&self) -> impl Iterator<Item = &M::Error> {
        self.rules()
            .unwrap_or(&[])
            .iter()
            .map(|applied| &applied.error)
    }

    /// The full rule errors, or `None` if the rejection did not come from rules.
    #[must_use]
    pub fn rules(&self) -> Option<&[AppliedError<M>]> {
        match self {
            Self::Rules(errors) => Some(errors),
            _ => None,
        }
    }
}

impl<M: Machine> Clone for Rejection<M>
where
    M::Error: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::UnknownEvent(event) => Self::UnknownEvent(*event),
            Self::NotAllowed { event, from } => Self::NotAllowed {
                event: *event,
                from: *from,
            },
            Self::Ambiguous(event) => Self::Ambiguous(*event),
            Self::Rules(errors) => Self::Rules(errors.clone()),
        }
    }
}

impl<M: Machine> PartialEq for Rejection<M>
where
    M::Error: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::UnknownEvent(a), Self::UnknownEvent(b)) => a == b,
            (
                Self::NotAllowed {
                    event: event_a,
                    from: from_a,
                },
                Self::NotAllowed {
                    event: event_b,
                    from: from_b,
                },
            ) => event_a == event_b && from_a == from_b,
            (Self::Ambiguous(a), Self::Ambiguous(b)) => a == b,
            (Self::Rules(a), Self::Rules(b)) => a == b,
            _ => false,
        }
    }
}

impl<M: Machine> Eq for Rejection<M> where M::Error: Eq {}

impl<M: Machine> fmt::Debug for Rejection<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(event) => f.debug_tuple("UnknownEvent").field(event).finish(),
            Self::NotAllowed { event, from } => f
                .debug_struct("NotAllowed")
                .field("event", event)
                .field("from", from)
                .finish(),
            Self::Ambiguous(event) => f.debug_tuple("Ambiguous").field(event).finish(),
            Self::Rules(errors) => f.debug_tuple("Rules").field(errors).finish(),
        }
    }
}

impl<M: Machine> fmt::Display for Rejection<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(event) => write!(f, "{} has no edge", M::event_name(*event)),
            Self::NotAllowed { event, from } => write!(
                f,
                "{} is not allowed from {}",
                M::event_name(*event),
                M::state_name(*from)
            ),
            Self::Ambiguous(event) => write!(
                f,
                "{} has several edges and none applies from the current state",
                M::event_name(*event)
            ),
            Self::Rules(errors) => {
                f.write_str("rejected by rules: ")?;
                for (index, applied) in errors.iter().enumerate() {
                    if index > 0 {
                        f.write_str("; ")?;
                    }
                    write!(f, "{applied}")?;
                }
                Ok(())
            }
        }
    }
}

impl<M: Machine> std::error::Error for Rejection<M> {}

/// Construction errors: detected at startup, not in production.
#[non_exhaustive]
pub enum BuildError<M: Machine> {
    /// Two edges for the same event start at the same state.
    Conflict {
        /// Duplicated event.
        event: M::Event,
        /// State from which it starts twice.
        from: M::State,
    },

    /// An event from `require_events` has no edge.
    MissingEvent(M::Event),

    /// A state from `require_states` does not appear in any edge and is not a root.
    MissingState(M::State),

    /// Several edges without origin for the same event: `execute_from_any` cannot
    /// choose.
    AmbiguousOriginless(M::Event),

    /// An edge without origin (`FROM: &[]`) can never be selected because every reachable
    /// state already declares an explicit edge for that event.
    ShadowedOriginless(M::Event),

    /// A state in the topology is not reachable from any declared root.
    UnreachableState(M::State),
}

impl<M: Machine> Clone for BuildError<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Machine> Copy for BuildError<M> {}

impl<M: Machine> PartialEq for BuildError<M> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Conflict {
                    event: event_a,
                    from: from_a,
                },
                Self::Conflict {
                    event: event_b,
                    from: from_b,
                },
            ) => event_a == event_b && from_a == from_b,
            (Self::MissingEvent(a), Self::MissingEvent(b)) => a == b,
            (Self::MissingState(a), Self::MissingState(b)) => a == b,
            (Self::AmbiguousOriginless(a), Self::AmbiguousOriginless(b)) => a == b,
            (Self::ShadowedOriginless(a), Self::ShadowedOriginless(b)) => a == b,
            (Self::UnreachableState(a), Self::UnreachableState(b)) => a == b,
            _ => false,
        }
    }
}

impl<M: Machine> Eq for BuildError<M> {}

impl<M: Machine> fmt::Debug for BuildError<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { event, from } => f
                .debug_struct("Conflict")
                .field("event", event)
                .field("from", from)
                .finish(),
            Self::MissingEvent(event) => f.debug_tuple("MissingEvent").field(event).finish(),
            Self::MissingState(state) => f.debug_tuple("MissingState").field(state).finish(),
            Self::AmbiguousOriginless(event) => {
                f.debug_tuple("AmbiguousOriginless").field(event).finish()
            }
            Self::ShadowedOriginless(event) => {
                f.debug_tuple("ShadowedOriginless").field(event).finish()
            }
            Self::UnreachableState(state) => {
                f.debug_tuple("UnreachableState").field(state).finish()
            }
        }
    }
}

impl<M: Machine> fmt::Display for BuildError<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { event, from } => write!(
                f,
                "{} has two edges from {}",
                M::event_name(*event),
                M::state_name(*from)
            ),
            Self::MissingEvent(event) => write!(f, "{} has no edge", M::event_name(*event)),
            Self::MissingState(state) => write!(
                f,
                "{} is not part of any edge and is not a root",
                M::state_name(*state)
            ),
            Self::AmbiguousOriginless(event) => write!(
                f,
                "{} has several edges without origin",
                M::event_name(*event)
            ),
            Self::ShadowedOriginless(event) => write!(
                f,
                "{} has an edge without origin shadowed in every reachable state",
                M::event_name(*event)
            ),
            Self::UnreachableState(state) => write!(
                f,
                "{} is not reachable from any declared root",
                M::state_name(*state)
            ),
        }
    }
}

impl<M: Machine> std::error::Error for BuildError<M> {}

/// All construction errors of a machine, reported together by `build()`.
///
/// Dereferences to `&[BuildError<M>]`, exposing slice methods such as `len`, `is_empty`,
/// `iter`, and indexing directly.
pub struct BuildErrors<M: Machine> {
    errors: Vec<BuildError<M>>,
}

impl<M: Machine> BuildErrors<M> {
    pub(crate) fn new(errors: Vec<BuildError<M>>) -> Self {
        Self { errors }
    }

    /// Consumes the container and returns the errors.
    #[must_use]
    pub fn into_vec(self) -> Vec<BuildError<M>> {
        self.errors
    }
}

impl<M: Machine> Deref for BuildErrors<M> {
    type Target = [BuildError<M>];

    fn deref(&self) -> &Self::Target {
        &self.errors
    }
}

impl<M: Machine> Clone for BuildErrors<M> {
    fn clone(&self) -> Self {
        Self {
            errors: self.errors.clone(),
        }
    }
}

impl<M: Machine> PartialEq for BuildErrors<M> {
    fn eq(&self, other: &Self) -> bool {
        self.errors == other.errors
    }
}

impl<M: Machine> Eq for BuildErrors<M> {}

impl<M: Machine> IntoIterator for BuildErrors<M> {
    type Item = BuildError<M>;
    type IntoIter = std::vec::IntoIter<BuildError<M>>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a, M: Machine> IntoIterator for &'a BuildErrors<M> {
    type Item = &'a BuildError<M>;
    type IntoIter = std::slice::Iter<'a, BuildError<M>>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}

impl<M: Machine> fmt::Debug for BuildErrors<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BuildErrors").field(&self.errors).finish()
    }
}

impl<M: Machine> fmt::Display for BuildErrors<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.errors.iter().enumerate() {
            if index > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl<M: Machine> std::error::Error for BuildErrors<M> {}

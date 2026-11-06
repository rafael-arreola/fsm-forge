//! The public errors, split by moment: execution ([`Rejection`]) and construction
//! ([`BuildError`] / [`BuildErrors`]).

use std::fmt;

use crate::machine::Machine;

/// Phase in which a rule rejected the transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// A precondition failed (`Rule::check`).
    Check,

    /// A post-condition failed (`Rule::verify`).
    Verify,
}

/// A rule error together with the rule and phase that produced it.
pub struct AppliedError<M: Machine> {
    /// Name of the rule that rejected.
    pub rule: &'static str,

    /// When it rejected.
    pub phase: Phase,

    /// The error itself.
    pub error: M::Error,
}

impl<M: Machine> fmt::Debug for AppliedError<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppliedError")
            .field("rule", &self.rule)
            .field("phase", &self.phase)
            .field("error", &format_args!("{}", self.error))
            .finish()
    }
}

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

    /// When overriding: the event has several edges and none starts at the current
    /// state.
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
    pub fn rules(&self) -> Option<&[AppliedError<M>]> {
        match self {
            Self::Rules(errors) => Some(errors),
            _ => None,
        }
    }
}

impl<M: Machine> fmt::Display for AppliedError<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}]: {}",
            self.rule,
            phase_name(self.phase),
            self.error
        )
    }
}

impl<M: Machine> fmt::Debug for Rejection<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
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
                "{} has several edges and none starts at the current state",
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

fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Check => "check",
        Phase::Verify => "verify",
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
    AmbiguousOriginless {
        /// Event with more than one edge without origin.
        event: M::Event,
    },

    /// A state in the topology is not reachable from any declared root.
    UnreachableState(M::State),
}

/// All construction errors of a machine, reported together by `build()`.
pub struct BuildErrors<M: Machine> {
    errors: Vec<BuildError<M>>,
}

impl<M: Machine> BuildErrors<M> {
    pub(crate) fn new(errors: Vec<BuildError<M>>) -> Self {
        Self { errors }
    }

    /// Iterates the errors in the order they were detected.
    pub fn iter(&self) -> std::slice::Iter<'_, BuildError<M>> {
        self.errors.iter()
    }

    /// The errors as a slice.
    pub fn as_slice(&self) -> &[BuildError<M>] {
        &self.errors
    }

    /// How many errors there are.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// `true` if there are no errors. In practice `build()` only returns `BuildErrors`
    /// when there is at least one.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Consumes the container and returns the errors.
    pub fn into_vec(self) -> Vec<BuildError<M>> {
        self.errors
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
            Self::AmbiguousOriginless { event } => write!(
                f,
                "{} has several edges without origin",
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

impl<M: Machine> fmt::Debug for BuildError<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<M: Machine> std::error::Error for BuildError<M> {}

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

impl<M: Machine> fmt::Debug for BuildErrors<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<M: Machine> std::error::Error for BuildErrors<M> {}

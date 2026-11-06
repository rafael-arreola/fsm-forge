use std::fmt::{Debug, Display};
use std::hash::Hash;

/// Describes a domain to the engine.
///
/// Implement it on a small type that represents the machine (`struct TicketMachine;`),
/// never on the entity itself: the machine fixes the types and the rules, while the
/// entity only holds data.
pub trait Machine: Sized + 'static {
    /// The machine's state. Usually a copyable `enum`.
    type State: Copy + Eq + Hash + Debug + Send + Sync + 'static;

    /// The event that requests a transition.
    type Event: Copy + Eq + Hash + Debug + Send + Sync + 'static;

    /// The entity whose state changes. The engine clones it and returns the modified copy.
    type Entity: Clone;

    /// Data carried by the event: payload, actor, metadata.
    type Input;

    /// Work a rule describes and the caller runs after saving.
    ///
    /// The engine does not run effects: it returns them in [`Outcome`](crate::Outcome)::effects.
    type Effect;

    /// Why a rule rejects a transition.
    ///
    /// It is shown to the end user, so it must be readable, not an internal code.
    type Error: Display + Send + Sync + 'static;

    /// Reads the entity's current state.
    fn state(entity: &Self::Entity) -> Self::State;

    /// Writes the target state. Called by the engine after applying the rules.
    fn set_state(entity: &mut Self::Entity, state: Self::State);

    /// Readable name of the state for diagrams and errors. Defaults to `Debug`.
    fn state_name(state: Self::State) -> String {
        format!("{state:?}")
    }

    /// Readable name of the event for diagrams and errors. Defaults to `Debug`.
    fn event_name(event: Self::Event) -> String {
        format!("{event:?}")
    }
}

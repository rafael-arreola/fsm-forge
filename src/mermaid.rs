use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Write};
use std::hash::Hash;
use std::iter::once;

use crate::{Engine, Machine};

impl<M: Machine> Engine<M> {
    /// `stateDiagram-v2` diagram generated from the declared edges.
    ///
    /// - Ids: the state's `Debug`, sanitized and disambiguated when two states collide.
    /// - Labels: `Machine::state_name` / `Machine::event_name`, escaped.
    /// - Edges without origin appear as a note on their target state.
    pub fn to_mermaid(&self) -> String {
        // Every state in the diagram, once, in order of appearance: roots first, then
        // the edges in declaration order.
        let mut seen = HashSet::new();
        let states: Vec<M::State> = self
            .roots
            .iter()
            .copied()
            .chain(
                self.edges
                    .iter()
                    .flat_map(|edge| edge.from.iter().copied().chain(once(edge.to))),
            )
            .filter(|state| seen.insert(*state))
            .collect();

        // `ids` has an entry for every state in `states`, and every state below comes
        // from the same roots and edges, so indexing cannot fail.
        let ids = assign_ids(&states);

        // Writing to a `String` cannot fail, so the `fmt::Result`s are ignored.
        let mut diagram = String::from("stateDiagram-v2\n");
        for state in &states {
            let name = label(&M::state_name(*state));
            let _ = writeln!(diagram, "    state \"{name}\" as {}", ids[state]);
        }
        for root in &self.roots {
            let _ = writeln!(diagram, "    [*] --> {}", ids[root]);
        }
        for edge in &self.edges {
            let event = label(&M::event_name(edge.event));
            let to = &ids[&edge.to];
            if edge.from.is_empty() {
                let _ = writeln!(diagram, "    note right of {to} : {event}");
            }
            for from in &edge.from {
                let _ = writeln!(diagram, "    {} --> {to} : {event}", ids[from]);
            }
        }
        diagram
    }
}

/// A unique Mermaid id per state, derived from its `Debug`.
///
/// When two states sanitize to the same id (`"a-b"` and `"a_b"` both become `a_b`), the
/// later one gets a numeric suffix: `a_b_2`, `a_b_3`...
fn assign_ids<T: Copy + Eq + Hash + Debug>(states: &[T]) -> HashMap<T, String> {
    let mut used = HashSet::new();
    states
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let base = id_base(state, index);
            let mut id = base.clone();
            let mut suffix = 2;
            while !used.insert(id.clone()) {
                id = format!("{base}_{suffix}");
                suffix += 1;
            }
            (*state, id)
        })
        .collect()
}

/// Id base: ASCII alphanumeric characters as-is, everything else `_`. Never empty.
fn id_base(state: &impl Debug, index: usize) -> String {
    let sanitized: String = format!("{state:?}")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        format!("state_{index}")
    } else {
        sanitized
    }
}

/// A Mermaid label cannot break the line or close the string.
fn label(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "'")
        .replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state whose `Debug` is fixed, to force collisions and empty ids.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct Raw(&'static str);

    impl Debug for Raw {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }

    #[test]
    fn ids_keep_alphanumerics_and_replace_the_rest() {
        let ids = assign_ids(&[Raw("Draft"), Raw("in review"), Raw("Ñandú")]);

        assert_eq!(ids[&Raw("Draft")], "Draft");
        assert_eq!(ids[&Raw("in review")], "in_review");
        assert_eq!(ids[&Raw("Ñandú")], "_and_");
    }

    #[test]
    fn colliding_ids_get_a_suffix_in_order_of_appearance() {
        let ids = assign_ids(&[Raw("a-b"), Raw("a_b"), Raw("a b")]);

        assert_eq!(ids[&Raw("a-b")], "a_b");
        assert_eq!(ids[&Raw("a_b")], "a_b_2");
        assert_eq!(ids[&Raw("a b")], "a_b_3");
    }

    #[test]
    fn a_suffix_never_reuses_an_id_already_taken() {
        // "a_b_2" is taken by the second state, so the collision of the third with
        // "a_b" must skip it.
        let ids = assign_ids(&[Raw("a_b"), Raw("a_b_2"), Raw("a-b")]);

        assert_eq!(ids[&Raw("a_b")], "a_b");
        assert_eq!(ids[&Raw("a_b_2")], "a_b_2");
        assert_eq!(ids[&Raw("a-b")], "a_b_3");
    }

    #[test]
    fn an_empty_debug_falls_back_to_the_position() {
        let ids = assign_ids(&[Raw("Open"), Raw("")]);

        assert_eq!(ids[&Raw("")], "state_1");
    }

    #[test]
    fn labels_cannot_close_the_string_or_break_the_line() {
        assert_eq!(label("say \"hi\""), "say 'hi'");
        assert_eq!(label("one\ntwo\r\nthree"), "one two  three");
        assert_eq!(label("a\\b"), "a\\\\b");
    }
}

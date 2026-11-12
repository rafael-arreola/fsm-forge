use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Write};
use std::hash::Hash;
use std::iter::once;

use crate::{Engine, Machine};

impl<M: Machine> Engine<M> {
    /// `stateDiagram-v2` diagram generated from the declared edges.
    ///
    /// - Ids: sanitized from the state's `Debug` and disambiguated on collisions or
    ///   reserved keywords.
    /// - Labels: `Machine::state_name` / `Machine::event_name`, sanitized for Mermaid.
    /// - Edges without origin appear as a note on their target state.
    #[must_use]
    pub fn to_mermaid(&self) -> String {
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

        let ids = assign_ids(&states);

        let mut diagram = String::from("stateDiagram-v2\n");
        for state in &states {
            let id = &ids[state];
            let name = state_label(&M::state_name(*state));
            let display = if name.trim().is_empty() {
                id.as_str()
            } else {
                name.as_str()
            };
            let _ = writeln!(diagram, "    state \"{display}\" as {id}");
        }
        for root in &self.roots {
            let _ = writeln!(diagram, "    [*] --> {}", ids[root]);
        }
        for edge in &self.edges {
            let event = event_label(&M::event_name(edge.event));
            let to = &ids[&edge.to];
            if edge.from.is_empty() {
                let note = if event.trim().is_empty() {
                    "(unnamed)"
                } else {
                    event.as_str()
                };
                let _ = writeln!(diagram, "    note right of {to} : {note}");
            }
            for from in &edge.from {
                if event.trim().is_empty() {
                    let _ = writeln!(diagram, "    {} --> {to}", ids[from]);
                } else {
                    let _ = writeln!(diagram, "    {} --> {to} : {event}", ids[from]);
                }
            }
        }
        diagram
    }
}

/// A unique Mermaid id per state, derived from its `Debug`.
fn assign_ids<T: Copy + Eq + Hash + Debug>(states: &[T]) -> HashMap<T, String> {
    let mut used = HashSet::with_capacity(states.len());
    states
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let base = id_base(state, index);
            let id = if used.contains(&base) {
                (2..)
                    .map(|suffix| format!("{base}_{suffix}"))
                    .find(|candidate| !used.contains(candidate))
                    .expect("finite set of used ids")
            } else {
                base
            };
            used.insert(id.clone());
            (*state, id)
        })
        .collect()
}

/// Id base: ASCII alphanumeric characters as-is, everything else `_`. Never empty or a
/// reserved `stateDiagram-v2` keyword.
fn id_base(state: &impl Debug, index: usize) -> String {
    let sanitized: String = format!("{state:?}")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if sanitized.trim_matches('_').is_empty() {
        format!("state_{index}")
    } else if matches!(
        sanitized.as_str(),
        "state" | "end" | "note" | "as" | "scale" | "class" | "classDef" | "style" | "direction"
    ) {
        format!("{sanitized}_state")
    } else {
        sanitized
    }
}

/// Sanitizes a quoted state label so it cannot close the string, start a comment,
/// terminate the statement, or break the line.
fn state_label(text: &str) -> String {
    text.replace('"', "'")
        .replace(';', ",")
        .replace("%%", "% %")
        .replace(['\n', '\r'], " ")
}

/// Sanitizes an unquoted transition or note label, also replacing `:` which splits
/// transition descriptions in `stateDiagram-v2`.
fn event_label(text: &str) -> String {
    state_label(text).replace(':', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let ids = assign_ids(&[Raw("a_b"), Raw("a_b_2"), Raw("a-b")]);

        assert_eq!(ids[&Raw("a_b")], "a_b");
        assert_eq!(ids[&Raw("a_b_2")], "a_b_2");
        assert_eq!(ids[&Raw("a-b")], "a_b_3");
    }

    #[test]
    fn an_empty_or_symbol_only_debug_falls_back_to_the_position() {
        let ids = assign_ids(&[Raw("Open"), Raw(""), Raw("---")]);

        assert_eq!(ids[&Raw("")], "state_1");
        assert_eq!(ids[&Raw("---")], "state_2");
    }

    #[test]
    fn reserved_mermaid_keywords_are_disambiguated() {
        let ids = assign_ids(&[
            Raw("end"),
            Raw("state"),
            Raw("note"),
            Raw("as"),
            Raw("class"),
            Raw("classDef"),
            Raw("scale"),
            Raw("style"),
            Raw("direction"),
            Raw("End"),
        ]);

        assert_eq!(ids[&Raw("end")], "end_state");
        assert_eq!(ids[&Raw("state")], "state_state");
        assert_eq!(ids[&Raw("note")], "note_state");
        assert_eq!(ids[&Raw("as")], "as_state");
        assert_eq!(ids[&Raw("class")], "class_state");
        assert_eq!(ids[&Raw("classDef")], "classDef_state");
        assert_eq!(ids[&Raw("scale")], "scale_state");
        assert_eq!(ids[&Raw("style")], "style_state");
        assert_eq!(ids[&Raw("direction")], "direction_state");
        assert_eq!(ids[&Raw("End")], "End");
    }

    #[test]
    fn labels_sanitize_quotes_newlines_semicolons_colons_and_comments() {
        assert_eq!(state_label("say \"hi\""), "say 'hi'");
        assert_eq!(state_label("one\ntwo\r\nthree"), "one two  three");
        assert_eq!(state_label("step 1; %%comment"), "step 1, % %comment");
        assert_eq!(event_label("Retry { max: 3 }"), "Retry { max, 3 }");
    }
}

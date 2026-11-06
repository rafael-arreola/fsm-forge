//! Rules: checks, mutations, postconditions, effects, atomicity, and phases.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use fsm_forge::{Edge, Engine, Machine, Phase, Rejection, Rule, Scope, Step, emit, guard};

// ─────────────────────────────────────────────────────────────────────────────
// Invoice: edge methods, a hand-written rule, and a failing verify
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InvoiceState {
    Draft,
    Issued,
    Paid,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InvoiceEvent {
    Issue,
    Pay,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Invoice {
    state: InvoiceState,
    total: i64,
    paid: i64,
    note: Option<&'static str>,
    cancelled: bool,
}

/// Invoice machine: checks, mutation, verify, effects, and a hand-written rule.
///
/// ```mermaid
/// stateDiagram-v2
///     state "Draft" as Draft
///     state "Issued" as Issued
///     state "Paid" as Paid
///     state "Cancelled" as Cancelled
///     [*] --> Draft
///     Draft --> Issued : Issue
///     Issued --> Paid : Pay
///     Draft --> Cancelled : Cancel
/// ```
struct InvoiceMachine;

impl Machine for InvoiceMachine {
    type State = InvoiceState;
    type Event = InvoiceEvent;
    type Entity = Invoice;
    type Input = i64;
    type Effect = &'static str;
    type Error = &'static str;

    fn state(invoice: &Invoice) -> InvoiceState {
        invoice.state
    }

    fn set_state(invoice: &mut Invoice, state: InvoiceState) {
        invoice.state = state;
    }
}

/// Hand-written rule: exercises all four phases.
struct ExactPayment;

impl Rule<InvoiceMachine> for ExactPayment {
    fn check(&self, step: &Step<'_, InvoiceMachine>) -> Result<(), &'static str> {
        if *step.input == step.entity.total - step.entity.paid {
            Ok(())
        } else {
            Err("payment does not cover the balance")
        }
    }

    fn apply(&self, step: &Step<'_, InvoiceMachine>, invoice: &mut Invoice) {
        invoice.paid += *step.input;
    }

    fn verify(
        &self,
        _step: &Step<'_, InvoiceMachine>,
        invoice: &Invoice,
    ) -> Result<(), &'static str> {
        if invoice.paid <= invoice.total {
            Ok(())
        } else {
            Err("invoice overpaid")
        }
    }

    fn effects(&self, _step: &Step<'_, InvoiceMachine>, _invoice: &Invoice) -> Vec<&'static str> {
        vec!["payment_recorded"]
    }
}

/// Counts how many times `effects` is called. Used to prove `diagnose` skips it.
struct CountEffects(Arc<AtomicUsize>);

impl Rule<InvoiceMachine> for CountEffects {
    fn effects(&self, _step: &Step<'_, InvoiceMachine>, _invoice: &Invoice) -> Vec<&'static str> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Vec::new()
    }
}

struct Issue;

impl Edge<InvoiceMachine> for Issue {
    const EVENT: InvoiceEvent = InvoiceEvent::Issue;
    const FROM: &'static [InvoiceState] = &[InvoiceState::Draft];
    const TO: InvoiceState = InvoiceState::Issued;
}

impl Rule<InvoiceMachine> for Issue {
    fn check(&self, step: &Step<'_, InvoiceMachine>) -> Result<(), &'static str> {
        if step.entity.total > 0 {
            Ok(())
        } else {
            Err("total must be positive")
        }
    }

    fn apply(&self, step: &Step<'_, InvoiceMachine>, invoice: &mut Invoice) {
        invoice.note = Some(if *step.input == 0 {
            "standard"
        } else {
            "discounted"
        });
    }

    fn verify(
        &self,
        _step: &Step<'_, InvoiceMachine>,
        invoice: &Invoice,
    ) -> Result<(), &'static str> {
        if invoice.note.is_some() {
            Ok(())
        } else {
            Err("note missing")
        }
    }

    fn effects(&self, _step: &Step<'_, InvoiceMachine>, _invoice: &Invoice) -> Vec<&'static str> {
        vec!["invoice.issued"]
    }
}

struct Pay;

impl Edge<InvoiceMachine> for Pay {
    const EVENT: InvoiceEvent = InvoiceEvent::Pay;
    const FROM: &'static [InvoiceState] = &[InvoiceState::Issued];
    const TO: InvoiceState = InvoiceState::Paid;
}

impl Rule<InvoiceMachine> for Pay {}

struct Cancel;

impl Edge<InvoiceMachine> for Cancel {
    const EVENT: InvoiceEvent = InvoiceEvent::Cancel;
    const FROM: &'static [InvoiceState] = &[InvoiceState::Draft];
    const TO: InvoiceState = InvoiceState::Cancelled;
}

impl Rule<InvoiceMachine> for Cancel {
    fn apply(&self, _step: &Step<'_, InvoiceMachine>, invoice: &mut Invoice) {
        invoice.cancelled = true;
    }

    fn verify(
        &self,
        _step: &Step<'_, InvoiceMachine>,
        invoice: &Invoice,
    ) -> Result<(), &'static str> {
        if invoice.cancelled {
            Err("cancel verification failed")
        } else {
            Ok(())
        }
    }
}

fn invoice_engine() -> Engine<InvoiceMachine> {
    Engine::<InvoiceMachine>::builder()
        .edge(Issue)
        .edge(Pay)
        .edge(Cancel)
        .rule(
            Scope::on(&[InvoiceEvent::Issue]),
            guard("non_negative_input", |step| {
                if *step.input >= 0 {
                    Ok(())
                } else {
                    Err("input must be non-negative")
                }
            }),
        )
        .rule(Scope::on(&[InvoiceEvent::Pay]), ExactPayment)
        .rule(Scope::always(), emit("audit", "audit"))
        .initial(InvoiceState::Draft)
        .require_events([InvoiceEvent::Issue, InvoiceEvent::Pay, InvoiceEvent::Cancel])
        .require_states([
            InvoiceState::Draft,
            InvoiceState::Issued,
            InvoiceState::Paid,
            InvoiceState::Cancelled,
        ])
        .build()
        .expect("valid invoice machine")
}

fn draft() -> Invoice {
    Invoice {
        state: InvoiceState::Draft,
        total: 100,
        paid: 0,
        note: None,
        cancelled: false,
    }
}

#[test]
fn issue_runs_check_apply_verify_and_effects() {
    let engine = invoice_engine();
    let invoice = draft();

    let outcome = engine
        .execute(&invoice, InvoiceEvent::Issue, &0)
        .expect("issue is valid");

    assert_eq!(outcome.transition.from, InvoiceState::Draft);
    assert_eq!(outcome.transition.to, InvoiceState::Issued);
    assert_eq!(outcome.entity.note, Some("standard"));
    assert_eq!(
        outcome.transition.rules,
        vec!["Issue", "non_negative_input", "audit"]
    );
    assert_eq!(
        outcome.effect_values().copied().collect::<Vec<_>>(),
        vec!["invoice.issued", "audit"]
    );
    assert_eq!(
        outcome.effects_of("Issue").copied().collect::<Vec<_>>(),
        vec!["invoice.issued"]
    );
    assert_eq!(outcome.effects[0].rule, "Issue");
    assert!(!outcome.transition.from_any);

    // The original entity never changes.
    assert_eq!(invoice.note, None);
    assert_eq!(invoice.state, InvoiceState::Draft);
}

#[test]
fn all_check_errors_are_collected_together() {
    let engine = invoice_engine();
    let invoice = Invoice {
        total: 0,
        ..draft()
    };

    let rejection = engine
        .execute(&invoice, InvoiceEvent::Issue, &-1)
        .expect_err("the edge check and the global guard must fail");

    let errors = rejection.rules().expect("rejection comes from rules");
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].rule, "Issue");
    assert_eq!(errors[0].phase, Phase::Check);
    assert_eq!(errors[0].error, "total must be positive");
    assert_eq!(errors[1].rule, "non_negative_input");
    assert_eq!(errors[1].phase, Phase::Check);
    assert_eq!(errors[1].error, "input must be non-negative");
    assert!(errors.iter().all(|error| error.phase == Phase::Check));
}

#[test]
fn verify_failure_discards_mutations_and_effects() {
    let engine = invoice_engine();
    let invoice = draft();

    let rejection = engine
        .execute(&invoice, InvoiceEvent::Cancel, &0)
        .expect_err("the verify method always fails");

    let errors = rejection.rules().expect("rejection comes from rules");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].rule, "Cancel");
    assert_eq!(errors[0].phase, Phase::Verify);
    assert!(!invoice.cancelled);
    assert_eq!(invoice.state, InvoiceState::Draft);
}

#[test]
fn hand_written_rule_runs_check_apply_verify_and_effects() {
    let engine = invoice_engine();
    let invoice = Invoice {
        state: InvoiceState::Issued,
        ..draft()
    };

    let outcome = engine
        .execute(&invoice, InvoiceEvent::Pay, &100)
        .expect("exact payment is valid");

    assert_eq!(outcome.entity.paid, 100);
    assert_eq!(outcome.entity.state, InvoiceState::Paid);
    assert_eq!(
        outcome.transition.rules,
        vec!["Pay", "ExactPayment", "audit"]
    );
    assert_eq!(
        outcome.effect_values().copied().collect::<Vec<_>>(),
        vec!["payment_recorded", "audit"]
    );
    assert_eq!(
        outcome
            .effects_of("ExactPayment")
            .copied()
            .collect::<Vec<_>>(),
        vec!["payment_recorded"]
    );
}

#[test]
fn wrong_payment_is_rejected_by_the_rule_name_and_phase() {
    let engine = invoice_engine();
    let invoice = Invoice {
        state: InvoiceState::Issued,
        ..draft()
    };

    let rejection = engine
        .execute(&invoice, InvoiceEvent::Pay, &30)
        .expect_err("30 does not cover the balance");

    let errors = rejection.rules().expect("rejection comes from rules");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].rule, "ExactPayment");
    assert_eq!(errors[0].phase, Phase::Check);
    assert_eq!(invoice.paid, 0);
}

#[test]
fn diagnose_does_not_run_effects() {
    let counter = Arc::new(AtomicUsize::new(0));
    let engine = Engine::<InvoiceMachine>::builder()
        .edge(Issue)
        .rule(Scope::always(), CountEffects(counter.clone()))
        .initial(InvoiceState::Draft)
        .build()
        .expect("valid invoice machine");
    let invoice = draft();

    let diagnosis = engine
        .diagnose(&invoice, InvoiceEvent::Issue, &0)
        .expect("diagnosis is valid");
    assert_eq!(diagnosis.to, InvoiceState::Issued);
    assert_eq!(diagnosis.rules, vec!["Issue", "CountEffects"]);
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let outcome = engine
        .execute(&invoice, InvoiceEvent::Issue, &0)
        .expect("execution is valid");
    assert_eq!(outcome.transition.to, InvoiceState::Issued);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn outcome_carries_the_result_and_is_not_a_self_loop() {
    let engine = invoice_engine();

    let outcome = engine
        .execute(&draft(), InvoiceEvent::Issue, &0)
        .expect("issue is valid");

    assert!(!outcome.transition.is_self_loop());
    assert_eq!(outcome.entity.state, InvoiceState::Issued);
    assert_eq!(outcome.entity.note, Some("standard"));
    assert_eq!(
        outcome.effect_values().copied().collect::<Vec<_>>(),
        vec!["invoice.issued", "audit"]
    );
}

#[test]
fn rejection_without_rules_has_no_rule_errors() {
    let engine = invoice_engine();

    let rejection: Rejection<InvoiceMachine> = engine
        .execute(&draft(), InvoiceEvent::Pay, &100)
        .expect_err("Pay is not allowed from Draft");

    assert!(rejection.rules().is_none());
    assert_eq!(rejection.rule_errors().count(), 0);
}

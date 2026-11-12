//! `Pending -> Paid`: exercises `check`, `apply`, `verify`, and `effects` in one edge.

use fsm_forge::{Edge, Step};

use crate::machine::{Order, OrderEvent, OrderMachine, OrderStatus};

pub struct PayOrder;

impl Edge<OrderMachine> for PayOrder {
    const EVENT: OrderEvent = OrderEvent::Pay;
    const FROM: &'static [OrderStatus] = &[OrderStatus::Pending];
    const TO: OrderStatus = OrderStatus::Paid;

    // 1. Precondition over the untouched entity and input.
    fn check(&self, step: &Step<'_, OrderMachine>) -> Result<(), &'static str> {
        if *step.input <= 0 {
            return Err("payment must be positive");
        }
        if *step.input < step.entity.total - step.entity.paid {
            return Err("payment does not cover the remaining balance");
        }
        Ok(())
    }

    // 2. Mutation on a cloned copy of the entity.
    fn apply(&self, step: &Step<'_, OrderMachine>, order: &mut Order) {
        order.paid += *step.input;
    }

    // 3. Post-condition over the mutated copy and destination state.
    fn verify(&self, _step: &Step<'_, OrderMachine>, order: &Order) -> Result<(), &'static str> {
        if order.paid <= order.total {
            Ok(())
        } else {
            Err("order was overpaid")
        }
    }

    // 4. Side effects described as values for the caller to run after saving.
    fn effects(&self, _step: &Step<'_, OrderMachine>, _order: &Order) -> Vec<&'static str> {
        vec!["send_payment_receipt"]
    }
}

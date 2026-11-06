//! `Pending -> Paid`: the payment must cover the balance and must not overpay.

use fsm_forge::{Edge, Rule, Step};

use crate::machine::{Order, OrderEvent, OrderMachine, OrderStatus};

pub struct PayOrder;

impl Edge<OrderMachine> for PayOrder {
    const EVENT: OrderEvent = OrderEvent::Pay;
    const FROM: &'static [OrderStatus] = &[OrderStatus::Pending];
    const TO: OrderStatus = OrderStatus::Paid;
}

impl Rule<OrderMachine> for PayOrder {
    fn check(&self, step: &Step<'_, OrderMachine>) -> Result<(), &'static str> {
        if *step.input <= 0 {
            return Err("the payment must be positive");
        }
        if *step.input != step.entity.total - step.entity.paid {
            return Err("the payment does not cover the balance");
        }
        Ok(())
    }

    fn apply(&self, step: &Step<'_, OrderMachine>, order: &mut Order) {
        order.paid += *step.input;
    }

    fn verify(&self, _step: &Step<'_, OrderMachine>, order: &Order) -> Result<(), &'static str> {
        if order.paid <= order.total {
            Ok(())
        } else {
            Err("the order was overpaid")
        }
    }

    fn effects(&self, _step: &Step<'_, OrderMachine>, _order: &Order) -> Vec<&'static str> {
        vec!["order.paid"]
    }
}

//! `Paid -> Shipped`: the order must be fully paid before shipping.

use fsm_forge::{Edge, Rule, Step};

use crate::machine::{Order, OrderEvent, OrderMachine, OrderStatus};

pub struct ShipOrder;

impl Edge<OrderMachine> for ShipOrder {
    const EVENT: OrderEvent = OrderEvent::Ship;
    const FROM: &'static [OrderStatus] = &[OrderStatus::Paid];
    const TO: OrderStatus = OrderStatus::Shipped;
}

impl Rule<OrderMachine> for ShipOrder {
    fn check(&self, step: &Step<'_, OrderMachine>) -> Result<(), &'static str> {
        if step.entity.paid == step.entity.total {
            Ok(())
        } else {
            Err("the order is not fully paid")
        }
    }

    fn effects(&self, _step: &Step<'_, OrderMachine>, _order: &Order) -> Vec<&'static str> {
        vec!["order.shipped"]
    }
}

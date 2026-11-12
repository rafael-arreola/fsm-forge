//! `Paid -> Shipped`: ship a fully paid order.

use fsm_forge::{Edge, Step};

use crate::machine::{Order, OrderEvent, OrderMachine, OrderStatus};

pub struct ShipOrder;

impl Edge<OrderMachine> for ShipOrder {
    const EVENT: OrderEvent = OrderEvent::Ship;
    const FROM: &'static [OrderStatus] = &[OrderStatus::Paid];
    const TO: OrderStatus = OrderStatus::Shipped;

    fn check(&self, step: &Step<'_, OrderMachine>) -> Result<(), &'static str> {
        if step.entity.paid == step.entity.total {
            Ok(())
        } else {
            Err("order is not fully paid")
        }
    }

    fn effects(&self, _step: &Step<'_, OrderMachine>, _order: &Order) -> Vec<&'static str> {
        vec!["notify_warehouse"]
    }
}

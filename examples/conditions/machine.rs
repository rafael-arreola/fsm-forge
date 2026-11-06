//! The order machine: domain types, the `Machine` contract, and the engine.

use fsm_forge::{BuildErrors, Engine, Machine};

use crate::edges::{PayOrder, ShipOrder};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    Pending,
    Paid,
    Shipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderEvent {
    Pay,
    Ship,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub status: OrderStatus,
    pub total: i64,
    pub paid: i64,
}

pub struct OrderMachine;

impl Machine for OrderMachine {
    type State = OrderStatus;
    type Event = OrderEvent;
    type Entity = Order;
    type Input = i64;
    type Effect = &'static str;
    type Error = &'static str;

    fn state(order: &Order) -> OrderStatus {
        order.status
    }

    fn set_state(order: &mut Order, status: OrderStatus) {
        order.status = status;
    }
}

pub fn order_engine() -> Result<Engine<OrderMachine>, BuildErrors<OrderMachine>> {
    Engine::<OrderMachine>::builder()
        .edge(PayOrder)
        .edge(ShipOrder)
        .initial(OrderStatus::Pending)
        .require_events([OrderEvent::Pay, OrderEvent::Ship])
        .require_states([
            OrderStatus::Pending,
            OrderStatus::Paid,
            OrderStatus::Shipped,
        ])
        .build()
}

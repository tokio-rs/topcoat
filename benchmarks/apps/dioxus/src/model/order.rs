use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// An order from `orders.json`.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Order {
    pub id: u32,
    pub number: String,
    pub customer: String,
    pub email: String,
    #[serde(with = "time::serde::rfc3339")]
    pub placed_at: OffsetDateTime,
    pub status: OrderStatus,
    pub items: u32,
    pub total_cents: u64,
    pub country: String,
}

/// The fulfilment state of an order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    Paid,
    Shipped,
    Delivered,
    Refunded,
    Cancelled,
}

impl OrderStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Paid => "Paid",
            Self::Shipped => "Shipped",
            Self::Delivered => "Delivered",
            Self::Refunded => "Refunded",
            Self::Cancelled => "Cancelled",
        }
    }

    /// Whether the order counts towards revenue.
    #[cfg(feature = "server")]
    pub fn is_revenue(self) -> bool {
        matches!(self, Self::Paid | Self::Shipped | Self::Delivered)
    }
}

/// Aggregates over all orders.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct OrderSummary {
    pub count: u64,
    pub revenue_cents: u64,
    pub average_cents: u64,
    pub refunded_cents: u64,
}

#[cfg(feature = "server")]
impl OrderSummary {
    pub fn new(orders: &[Order]) -> Self {
        let mut revenue_cents = 0;
        let mut revenue_orders = 0;
        let mut refunded_cents = 0;
        for order in orders {
            if order.status.is_revenue() {
                revenue_cents += order.total_cents;
                revenue_orders += 1;
            } else if order.status == OrderStatus::Refunded {
                refunded_cents += order.total_cents;
            }
        }
        let average_cents = match revenue_orders {
            0 => 0,
            n => (revenue_cents + n / 2) / n,
        };
        Self {
            count: orders.len() as u64,
            revenue_cents,
            average_cents,
            refunded_cents,
        }
    }
}

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// An order from `orders.json`.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
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
    #[cfg(feature = "ssr")]
    pub fn is_revenue(self) -> bool {
        matches!(self, Self::Paid | Self::Shipped | Self::Delivered)
    }
}

/// Aggregates over all orders, computed once at startup like a database query would.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OrderSummary {
    pub count: usize,
    pub revenue_cents: u64,
    pub average_cents: u64,
    pub refunded_cents: u64,
}

impl OrderSummary {
    #[cfg(feature = "ssr")]
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
            count: orders.len(),
            revenue_cents,
            average_cents,
            refunded_cents,
        }
    }
}

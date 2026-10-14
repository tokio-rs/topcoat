use std::str::FromStr;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use toasty::{
    schema::Model,
    stmt::{Expr, List},
};
use topcoat::{Result, context::Cx};

use super::{
    catalog::{Money, Product},
    customer::Customer,
};
use crate::db::{contains_pattern, db, latency};

#[derive(Debug, toasty::Model)]
#[index(status)]
pub struct Order {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub customer_id: u64,

    #[belongs_to]
    pub customer: toasty::Deferred<Customer>,

    #[has_many]
    pub items: toasty::Deferred<Vec<OrderItem>>,

    pub status: OrderStatus,

    /// Stored total so list pages do not need to fetch line items to calculate it.
    pub total: Money,

    pub placed_at: Timestamp,
}

impl Order {
    pub async fn find(cx: &Cx, id: u64) -> Result<Option<Order>> {
        let order = Order::filter_by_id(id).first().exec(&mut db(cx)).await?;
        Ok(order)
    }

    /// Loads a customer's orders and their items, newest first.
    pub async fn of_customer(cx: &Cx, id: u64) -> Result<Vec<Order>> {
        latency().await;
        let orders = Order::filter_by_customer_id(id)
            .include(Order::fields().items())
            .order_by(Order::fields().id().desc())
            .exec(&mut db(cx))
            .await?;
        Ok(orders)
    }

    /// Counts units across all line items. The items must already be loaded.
    pub fn units(&self) -> u32 {
        self.items.get().iter().map(|item| item.quantity).sum()
    }

    /// Loads up to `limit` pending or paid orders, oldest first, with their customers.
    pub async fn awaiting_action(cx: &Cx, limit: usize) -> Result<Vec<Order>> {
        latency().await;
        let status = Order::fields().status();
        let orders = Order::filter(
            status
                .eq(OrderStatus::Pending)
                .or(status.eq(OrderStatus::Paid)),
        )
        .include(Order::fields().customer())
        .order_by(Order::fields().placed_at().asc())
        .limit(limit)
        .exec(&mut db(cx))
        .await?;
        Ok(orders)
    }

    /// Changes the status if the transition is allowed. Returns `false` without
    /// changing the order for an invalid transition.
    pub async fn advance(&mut self, cx: &Cx, status: OrderStatus) -> Result<bool> {
        if !self.status.next().contains(&status) {
            return Ok(false);
        }
        self.update().status(status).exec(&mut db(cx)).await?;
        Ok(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, toasty::Embed)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    /// Awaiting payment.
    Pending,
    /// Payment received; awaiting shipment.
    Paid,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
}

impl OrderStatus {
    pub const ALL: [Self; 6] = [
        Self::Pending,
        Self::Paid,
        Self::Shipped,
        Self::Delivered,
        Self::Cancelled,
        Self::Refunded,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Paid => "Paid",
            Self::Shipped => "Shipped",
            Self::Delivered => "Delivered",
            Self::Cancelled => "Cancelled",
            Self::Refunded => "Refunded",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Paid => "paid",
            Self::Shipped => "shipped",
            Self::Delivered => "delivered",
            Self::Cancelled => "cancelled",
            Self::Refunded => "refunded",
        }
    }

    pub fn action(self) -> &'static str {
        match self {
            Self::Pending => "Mark as pending",
            Self::Paid => "Mark as paid",
            Self::Shipped => "Mark as shipped",
            Self::Delivered => "Mark as delivered",
            Self::Cancelled => "Cancel order",
            Self::Refunded => "Refund order",
        }
    }

    pub fn is_reversal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Refunded)
    }

    pub fn next(self) -> &'static [Self] {
        match self {
            Self::Pending => &[Self::Paid, Self::Cancelled],
            Self::Paid => &[Self::Shipped, Self::Refunded],
            Self::Shipped => &[Self::Delivered],
            Self::Delivered | Self::Cancelled | Self::Refunded => &[],
        }
    }
}

impl FromStr for OrderStatus {
    type Err = UnknownStatus;

    fn from_str(slug: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.slug() == slug)
            .ok_or(UnknownStatus)
    }
}

#[derive(Debug)]
pub struct UnknownStatus;

#[derive(Debug, toasty::Model)]
pub struct OrderItem {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub order_id: u64,

    #[belongs_to]
    pub order: toasty::Deferred<Order>,

    #[index]
    pub product_id: u64,

    #[belongs_to]
    pub product: toasty::Deferred<Product>,

    pub quantity: u32,

    /// Unit price at purchase time. Later catalog price changes do not affect it.
    pub unit_price: Money,
}

impl OrderItem {
    /// Loads an order's line items and their products.
    pub async fn of_order(cx: &Cx, order_id: u64) -> Result<Vec<OrderItem>> {
        latency().await;
        let items = OrderItem::filter_by_order_id(order_id)
            .include(OrderItem::fields().product())
            .order_by(OrderItem::fields().id().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(items)
    }

    pub fn subtotal(&self) -> Money {
        Money(self.unit_price.0 * i64::from(self.quantity))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderSort {
    #[default]
    Newest,
    Oldest,
    HighestTotal,
    LowestTotal,
}

impl OrderSort {
    pub const ALL: [Self; 4] = [
        Self::Newest,
        Self::Oldest,
        Self::HighestTotal,
        Self::LowestTotal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Newest => "Newest first",
            Self::Oldest => "Oldest first",
            Self::HighestTotal => "Highest total",
            Self::LowestTotal => "Lowest total",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Newest => "newest",
            Self::Oldest => "oldest",
            Self::HighestTotal => "highest-total",
            Self::LowestTotal => "lowest-total",
        }
    }
}

impl FromStr for OrderSort {
    type Err = UnknownSort;

    fn from_str(slug: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|sort| sort.slug() == slug)
            .ok_or(UnknownSort)
    }
}

#[derive(Debug)]
pub struct UnknownSort;

#[derive(Debug, Default)]
pub struct OrderSearch<'a> {
    pub status: Option<OrderStatus>,
    /// Searches customer names, email addresses, and order numbers.
    /// Blank text matches all orders.
    pub text: &'a str,
}

impl OrderSearch<'_> {
    /// Loads one page of matching orders, including customers and line items.
    /// Page numbers start at 1.
    pub async fn page(
        &self,
        cx: &Cx,
        sort: OrderSort,
        page: usize,
        per_page: usize,
    ) -> Result<Vec<Order>> {
        latency().await;

        let orders = self
            .query()
            .include(Order::fields().customer())
            .include(Order::fields().items())
            .order_by(match sort {
                // Seeded orders have IDs in date order.
                OrderSort::Newest => Order::fields().id().desc(),
                OrderSort::Oldest => Order::fields().id().asc(),
                OrderSort::HighestTotal => Order::fields().total().desc(),
                OrderSort::LowestTotal => Order::fields().total().asc(),
            })
            .limit(per_page)
            .offset(page.saturating_sub(1) * per_page)
            .exec(&mut db(cx))
            .await?;
        Ok(orders)
    }

    pub async fn count(&self, cx: &Cx) -> Result<usize> {
        let count = self.query().count().exec(&mut db(cx)).await?;
        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    fn query(&self) -> <Order as Model>::Query<List<Order>> {
        let status = self
            .status
            .map(|status| Order::fields().status().eq(status));

        let text = self.text.trim();
        let text = (!text.is_empty()).then(|| {
            let pattern = contains_pattern(text);
            let customer = Order::fields().customer();
            let mut matches = customer
                .name()
                .like_with_escape(pattern.clone(), '\\')
                .or(customer.email().like_with_escape(pattern, '\\'));
            if let Ok(id) = text.trim_start_matches('#').parse::<u64>() {
                matches = matches.or(Order::fields().id().eq(id));
            }
            matches
        });

        match [status, text].into_iter().flatten().reduce(Expr::and) {
            Some(filter) => Order::filter(filter),
            None => Order::all(),
        }
    }
}

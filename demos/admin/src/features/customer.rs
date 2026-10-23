use std::str::FromStr;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use toasty::{schema::Model, stmt::List};
use topcoat::{Result, context::Cx};

use super::{catalog::Money, order::Order};
use crate::db::{contains_pattern, db, latency};

#[derive(Debug, toasty::Model)]
pub struct Customer {
    #[key]
    #[auto]
    pub id: u64,

    pub name: String,

    #[unique]
    pub email: String,

    pub city: String,

    pub joined_at: Timestamp,

    #[has_many]
    pub orders: toasty::Deferred<Vec<Order>>,
}

impl Customer {
    pub async fn get(cx: &Cx, id: u64) -> Result<Customer> {
        latency().await;
        let customer = Customer::get_by_id(&mut db(cx), &id).await?;
        Ok(customer)
    }

    pub async fn find(cx: &Cx, id: u64) -> Result<Option<Customer>> {
        let customer = Customer::filter_by_id(id).first().exec(&mut db(cx)).await?;
        Ok(customer)
    }

    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spending {
    /// Counts all orders, including cancelled and refunded orders.
    pub orders: usize,
    /// Total spending, excluding cancelled and refunded orders.
    pub spent: Money,
    /// Number of orders included in `spent`.
    kept: i64,
}

impl Spending {
    pub fn of(orders: &[Order]) -> Self {
        let kept = orders.iter().filter(|order| !order.status.is_reversal());
        Self {
            orders: orders.len(),
            spent: kept.clone().map(|order| order.total).sum(),
            kept: i64::try_from(kept.count()).unwrap_or(i64::MAX),
        }
    }

    /// Average spending per order, excluding cancellations and refunds.
    /// Returns `None` if no orders remain.
    pub fn average_order(self) -> Option<Money> {
        (self.kept > 0).then(|| Money(self.spent.0 / self.kept))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CustomerSort {
    #[default]
    Name,
    Newest,
    Oldest,
}

impl CustomerSort {
    pub const ALL: [Self; 3] = [Self::Name, Self::Newest, Self::Oldest];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Newest => "newest",
            Self::Oldest => "oldest",
        }
    }
}

impl FromStr for CustomerSort {
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
pub struct CustomerSearch<'a> {
    /// Searches names and email addresses. Blank text matches all customers.
    pub text: &'a str,
}

impl CustomerSearch<'_> {
    /// Loads one page of matching customers and their orders. Page numbers start at 1.
    pub async fn page(
        &self,
        cx: &Cx,
        sort: CustomerSort,
        page: usize,
        per_page: usize,
    ) -> Result<Vec<Customer>> {
        latency().await;

        let customers = self
            .query()
            .include(Customer::fields().orders())
            .order_by(match sort {
                CustomerSort::Name => Customer::fields().name().asc(),
                CustomerSort::Newest => Customer::fields().joined_at().desc(),
                CustomerSort::Oldest => Customer::fields().joined_at().asc(),
            })
            .limit(per_page)
            .offset(page.saturating_sub(1) * per_page)
            .exec(&mut db(cx))
            .await?;
        Ok(customers)
    }

    pub async fn count(&self, cx: &Cx) -> Result<usize> {
        let count = self.query().count().exec(&mut db(cx)).await?;
        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    fn query(&self) -> <Customer as Model>::Query<List<Customer>> {
        let text = self.text.trim();
        if text.is_empty() {
            return Customer::all();
        }
        let pattern = contains_pattern(text);
        let fields = Customer::fields();
        Customer::filter(
            fields
                .name()
                .like_with_escape(pattern.clone(), '\\')
                .or(fields.email().like_with_escape(pattern, '\\')),
        )
    }
}

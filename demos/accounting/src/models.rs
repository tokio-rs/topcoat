use time::{Date, Duration, OffsetDateTime, macros::format_description};
use toasty::Db;
use topcoat::{
    Result,
    context::{Cx, app_context},
};

#[derive(Debug, toasty::Model)]
pub struct Invoice {
    #[key]
    pub id: String,
    pub customer: String,
    pub email: String,
    pub address: String,
    pub issued: String,
    pub due: String,
    pub notes: String,
    pub paid: bool,
    pub subtotal: i64,
    pub tax_percent: i64,
    #[has_many]
    pub lines: toasty::Deferred<Vec<InvoiceLine>>,
}

impl Invoice {
    pub fn total(&self) -> i64 {
        self.subtotal + tax(self.subtotal, self.tax_percent)
    }

    pub fn status(&self) -> &'static str {
        if self.paid {
            "Paid"
        } else if self.due < today() {
            "Overdue"
        } else {
            "Outstanding"
        }
    }
}

#[derive(Debug, toasty::Model)]
pub struct InvoiceLine {
    #[key]
    #[auto]
    pub id: u64,
    #[index]
    pub invoice_id: String,
    #[belongs_to(key = invoice_id, references = id)]
    pub invoice: toasty::Deferred<Invoice>,
    pub position: i64,
    pub description: String,
    pub quantity: i64,
    pub unit_price: i64,
}

pub fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

pub async fn invoices(cx: &Cx) -> Result<Vec<Invoice>> {
    Ok(Invoice::all()
        .order_by(Invoice::fields().issued().desc())
        .exec(&mut db(cx))
        .await?)
}

pub fn today() -> String {
    date(OffsetDateTime::now_utc().date())
}

pub fn date(value: Date) -> String {
    value
        .format(format_description!("[year]-[month]-[day]"))
        .unwrap()
}

pub fn money(cents: i64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

// Round tax once per invoice, to the nearest cent.
pub fn tax(subtotal: i64, percent: i64) -> i64 {
    (subtotal * percent + 50) / 100
}

pub async fn seed(db: &mut Db) -> toasty::Result<()> {
    let now = OffsetDateTime::now_utc().date();
    for (index, (customer, description, price, age, paid)) in [
        ("Acme Studio", "Brand identity", 240_000, 5, false),
        ("Northstar Labs", "Website development", 480_000, 40, false),
        ("Paper & Pine", "Editorial design", 125_000, 8, true),
        ("Olive Works", "Monthly retainer", 180_000, 12, false),
        ("Common Ground", "Product photography", 95_000, 38, false),
        ("Forma Architecture", "Portfolio website", 320_000, 21, true),
        ("Acme Studio", "Print production", 75_000, 48, true),
        ("Northstar Labs", "Design systems", 560_000, 68, true),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("INV-{}", 1001 + index);
        let issued = date(now - Duration::days(age));
        let due = date(now - Duration::days(age) + Duration::days(30));
        toasty::create!(Invoice {
            id: id.clone(), customer, email: "accounts@example.com",
            address: "24 Market Street\nPortland, OR 97205",
            issued, due, notes: "Thank you for your business. Payment is due within 30 days.",
            paid, subtotal: price, tax_percent: 10,
            lines: [{ position: 0, description, quantity: 1, unit_price: price }],
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

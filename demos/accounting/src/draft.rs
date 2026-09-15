use std::collections::HashMap;

use time::{Date, macros::format_description};
use topcoat::Result;

use crate::models::{Invoice, InvoiceLine};

pub const MAX_LINES: usize = 20;

pub struct Draft {
    pub customer: String,
    pub email: String,
    pub address: String,
    pub issued: String,
    pub due: String,
    pub notes: String,
    pub tax_percent: i64,
    pub lines: Vec<Line>,
}

impl Draft {
    pub fn from_form(form: &HashMap<String, String>) -> std::result::Result<Self, String> {
        let text = |name: &str| form.get(name).map_or("", String::as_str).trim().to_owned();
        let customer = text("customer");
        let email = text("email");
        let address = text("address");
        let issued = text("issued");
        let due = text("due");
        let notes = text("notes");
        if customer.is_empty() || customer.len() > 200 {
            return Err("Enter a customer name of up to 200 characters.".into());
        }
        if !email.contains('@') || email.len() > 254 {
            return Err("Enter a valid billing email.".into());
        }
        if address.len() > 1000 || notes.len() > 2000 {
            return Err("The address or notes are too long.".into());
        }
        let format = format_description!("[year]-[month]-[day]");
        let issue_date = Date::parse(&issued, format).map_err(|_| "Enter an issue date.")?;
        let due_date = Date::parse(&due, format).map_err(|_| "Enter a due date.")?;
        if due_date < issue_date {
            return Err("The due date must be on or after the issue date.".into());
        }
        let tax_percent = text("tax")
            .parse::<i64>()
            .ok()
            .filter(|value| (0..=100).contains(value))
            .ok_or("Tax must be a whole percentage from 0 to 100.")?;
        let keys: Vec<&str> = form
            .get("line-keys")
            .map_or("", String::as_str)
            .split(',')
            .collect();
        if keys.is_empty() || keys.len() > MAX_LINES {
            return Err("An invoice needs between 1 and 20 lines.".into());
        }
        let mut seen = std::collections::HashSet::new();
        let mut lines = Vec::new();
        for key in keys {
            if key.parse::<usize>().is_err() || !seen.insert(key) {
                return Err("Invalid invoice lines.".into());
            }
            let description = text(&format!("description-{key}"));
            if description.is_empty() || description.len() > 300 {
                return Err("Each line needs a description of up to 300 characters.".into());
            }
            lines.push(Line::parse(
                description,
                &text(&format!("quantity-{key}")),
                &text(&format!("price-{key}")),
            )?);
        }
        Ok(Self {
            customer,
            email,
            address,
            issued,
            due,
            notes,
            tax_percent,
            lines,
        })
    }

    pub async fn save(self, db: &mut toasty::Db) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let subtotal: i64 = self.lines.iter().map(Line::total).sum();
        // The invoice and its lines either all commit or all roll back.
        let mut tx = db.transaction().await?;
        toasty::create!(Invoice {
            id: id.clone(),
            customer: self.customer,
            email: self.email,
            address: self.address,
            issued: self.issued,
            due: self.due,
            notes: self.notes,
            paid: false,
            subtotal,
            tax_percent: self.tax_percent,
        })
        .exec(&mut tx)
        .await?;
        for (position, line) in self.lines.into_iter().enumerate() {
            toasty::create!(InvoiceLine {
                invoice_id: id.clone(),
                position: i64::try_from(position).unwrap(),
                description: line.description,
                quantity: line.quantity,
                unit_price: line.unit_price,
            })
            .exec(&mut tx)
            .await?;
        }
        tx.commit().await?;
        Ok(id)
    }
}

pub struct Line {
    pub description: String,
    pub quantity: i64,
    pub unit_price: i64,
}

impl Line {
    pub fn parse(
        description: String,
        quantity: &str,
        price: &str,
    ) -> std::result::Result<Self, String> {
        let quantity = quantity
            .parse::<i64>()
            .ok()
            .filter(|value| (1..=10_000).contains(value))
            .ok_or("Quantity must be a whole number from 1 to 10000.")?;
        let unit_price = cents(price)
            .ok_or("Enter a price from 0 to 1000000, with at most two decimal places.")?;
        Ok(Self {
            description,
            quantity,
            unit_price,
        })
    }

    pub fn total(&self) -> i64 {
        self.quantity * self.unit_price
    }
}

// Parse decimal money without a floating-point conversion.
fn cents(value: &str) -> Option<i64> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let fraction = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()? * 10,
        _ => fraction.parse::<i64>().ok()?,
    };
    whole
        .checked_mul(100)?
        .checked_add(fraction)
        .filter(|value| *value <= 100_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_is_exact_and_bounded() {
        assert_eq!(cents("19.99"), Some(1999));
        assert_eq!(cents("0.1"), Some(10));
        assert_eq!(cents("1000000"), Some(100_000_000));
        for invalid in [
            "",
            "-1",
            "NaN",
            "1.001",
            "1e3",
            "1000000.01",
            "999999999999999999",
        ] {
            assert_eq!(cents(invalid), None, "{invalid}");
        }
        assert!(Line::parse(String::new(), "0", "1").is_err());
        assert!(Line::parse(String::new(), "1.5", "1").is_err());
        assert_eq!(crate::models::tax(105, 10), 11);
    }
}

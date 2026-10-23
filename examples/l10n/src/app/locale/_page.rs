use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

use crate::app::t;

#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! { (t!(current_date, date = "kek")) })
}

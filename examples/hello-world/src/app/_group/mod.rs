use topcoat::{
    router::{Result, Slot, layout},
    view::view,
};

mod contact;

#[layout]
async fn group_layout(slot: Slot) -> Result {
    view! { (slot.await?) <div>"(in group layout)"</div> }
}

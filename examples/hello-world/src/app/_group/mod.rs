use topcoat::{
    router::{Slot, layout},
    view::{View, view},
};

mod contact;

#[layout]
async fn group_layout(slot: Slot) -> View {
    view! { (slot.await) <div>"(in group layout)"</div> }
}

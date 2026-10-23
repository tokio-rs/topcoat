use topcoat::{
    context::Cx,
    runtime::{DialogElement, ElementRef, HtmlElement, InputElement},
    view::element_id,
};
use topcoat_runtime_coherence::coherent;

#[test]
fn element_reference_metadata_agrees_without_a_dom() {
    let cx = Cx::default();
    let input = ElementRef::<InputElement>::new(element_id(&cx));
    let dialog = ElementRef::<DialogElement>::new(element_id(&cx));
    let html = ElementRef::<HtmlElement>::new(element_id(&cx));
    coherent!(input.id().to_owned());
    coherent!(dialog.clone().id().to_owned());
    coherent!(html.clone().id() == html.id());
    let borrowed = &input;
    coherent!(borrowed.clone().id().to_owned());
}

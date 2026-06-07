use topcoat::{
    context::Cx,
    runtime::{ReadSignal, Signal, SignalDeclaration, SignalId, Surrogate, Surrogated},
    view::{NodeViewParts, View, ViewParts},
};

fn render(parts: ViewParts) -> String {
    View::new(parts).render(&Cx::empty())
}

#[test]
fn primitive_surrogates_serialize_as_tagged_json() {
    let encoded = serde_json::to_string(&5_i32.into_surrogate()).unwrap();
    assert_eq!(encoded, r#"{"t":"i32","v":5}"#);

    let decoded: topcoat::runtime::I32 = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.into_real(), 5);
}

#[test]
fn primitive_surrogate_deserialization_rejects_wrong_tag() {
    let decoded = serde_json::from_str::<topcoat::runtime::F64>(r#"{"t":"i32","v":5}"#);
    assert!(decoded.is_err());
}

#[test]
fn signal_declaration_renders_tagged_payload() {
    let signal = Signal::new(1.0);
    let mut parts = ViewParts::new();
    SignalDeclaration::new(&signal).into_view_parts(&mut parts);

    let html = render(parts);
    assert!(html.starts_with(r#"<!-- ::topcoat::signal({"t":"signal""#));
    assert!(html.contains(r#""v":{"t":"f64","v":1.0}"#));
    assert!(html.ends_with(") -->"));
}

#[test]
fn read_signal_deserializes_tagged_value() {
    let id = serde_json::to_string(&SignalId::new()).unwrap();
    let encoded = format!(r#"[{{"id":{id},"value":{{"t":"f64","v":3.5}}}}]"#);

    let decoded: (ReadSignal<f64>,) = serde_json::from_str(&encoded).unwrap();
    assert_eq!(*decoded.0, 3.5);
}

#[test]
fn read_signal_deserialization_rejects_wrong_value_tag() {
    let id = serde_json::to_string(&SignalId::new()).unwrap();
    let encoded = format!(r#"[{{"id":{id},"value":{{"t":"i32","v":3}}}}]"#);

    let decoded = serde_json::from_str::<(ReadSignal<f64>,)>(&encoded);
    assert!(decoded.is_err());
}

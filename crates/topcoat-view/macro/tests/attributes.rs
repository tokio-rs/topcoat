#[test]
fn attributes_macro_builds_runtime_attributes() {
    let id = "submit";
    let dynamic = [
        ("data-skip", "skip"),
        ("data-state", "ready"),
        ("data-stop", "stop"),
        ("data-after", "after"),
    ];

    let mut attrs = topcoat::view::attributes! {
        class="button"
        id=(id)
        :data-bound=$(id.to_owned())
        @input="(e) => console.log(e)"
        if true {
            aria-label="Submit"
        } else {
            aria-label="Disabled"
        }
        for (key, value) in dynamic {
            if value == "skip" {
                continue;
            }
            if value == "stop" {
                break;
            }
            (key)=(value)
        }
        match id {
            "submit" => type="submit",
            _ => type="button",
        }
    };

    assert!(attrs.contains_key("class"));
    assert!(attrs.contains_key("id"));
    assert!(attrs.contains_key("aria-label"));
    assert!(attrs.contains_key("data-state"));
    assert!(attrs.contains_key("type"));
    assert!(attrs.contains_key("data-bound"));
    assert!(attrs.contains_key("data-topcoat-bind:data-bound"));
    assert!(attrs.contains_key("data-topcoat-on:input"));
    assert!(!attrs.contains_key("data-skip"));
    assert!(!attrs.contains_key("data-stop"));
    assert!(!attrs.contains_key("data-after"));
    assert!(attrs.get("missing").is_none());
}

#[tokio::test]
async fn spread_inserts_attribute_fragment_into_element() {
    use topcoat::{context::Cx, view::view};

    let attrs = topcoat::view::attributes! { type="submit" };
    let result: topcoat::Result = view! { <button (attrs)>"Save"</button> };
    let html = result.unwrap().render(&Cx::empty());

    assert_eq!(html, r#"<button type="submit">Save</button>"#);
}

#[tokio::test]
async fn spread_follows_other_attributes() {
    use topcoat::{context::Cx, view::view};

    let attrs = topcoat::view::attributes! { type="submit" };
    let result: topcoat::Result = view! { <button class="btn" (attrs)>"Save"</button> };
    let html = result.unwrap().render(&Cx::empty());

    assert!(html.contains(r#"class="btn""#));
    assert!(html.contains(r#"type="submit""#));
}

#[test]
fn dynamic_key_still_parses_after_spread_support() {
    // A parenthesized key followed by `=` remains a dynamic attribute, not a
    // spread.
    let name = "data-state";
    let attrs = topcoat::view::attributes! { (name)="ready" };
    assert!(attrs.contains_key("data-state"));
}

#[tokio::test]
async fn spread_merges_within_attributes_macro() {
    use topcoat::{context::Cx, view::view};

    let base = topcoat::view::attributes! { class="btn" type="button" };
    let merged = topcoat::view::attributes! { class="card" (base) };

    assert!(merged.contains_key("type"));

    // The spread's keys replace earlier ones, so `class` renders as `btn`.
    let result: topcoat::Result = view! { <div (merged)></div> };
    let html = result.unwrap().render(&Cx::empty());
    assert!(html.contains(r#"class="btn""#));
    assert!(!html.contains(r#"class="card""#));
}

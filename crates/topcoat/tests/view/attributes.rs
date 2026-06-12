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
        if true { aria-label="Submit" } else { aria-label="Disabled" }
        for (key, value) in dynamic {
            if value == "skip" { continue; }
            if value == "stop" { break; }
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

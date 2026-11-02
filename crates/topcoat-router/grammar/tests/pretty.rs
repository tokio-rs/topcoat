use topcoat_core_grammar::pretty::{Registry, pretty_print_str};
use topcoat_router_grammar::{
    layer::LayerAttr, layout::LayoutAttr, module_param::ModuleParam, not_found::NotFound,
    page::PageAttr, path_param::PathParam, query_params::QueryParamsAttr, route::RouteAttr,
    segment::Segment,
};

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register_attribute::<PageAttr>("page")
        .register_attribute::<RouteAttr>("route")
        .register_attribute::<LayoutAttr>("layout")
        .register_attribute::<LayerAttr>("layer")
        .register_attribute::<QueryParamsAttr>("query_params")
        .register_function_like::<NotFound>("not_found")
        .register_function_like::<Segment>("segment")
        .register_function_like::<PathParam>("path_param")
        .register_function_like::<ModuleParam>("module_param");
    registry
}

fn format(input: &str) -> String {
    pretty_print_str(&registry(), input).unwrap_or_else(|errors| {
        panic!(
            "failed to format input:\n{input}\nerror: {}",
            errors.first().unwrap()
        )
    })
}

/// Formatting `input` must produce `expected`, and formatting `expected` again
/// must be a fixed point.
fn assert_format(input: &str, expected: &str) {
    assert_eq!(format(input), expected, "formatting:\n{input}");
    assert_eq!(format(expected), expected, "reformatting:\n{expected}");
}

#[test]
fn separates_a_method_and_a_path_with_one_space() {
    assert_format(
        "#[route(GET   \"./report.csv\")]\nfn report() {}",
        "#[route(GET \"./report.csv\")]\nfn report() {}",
    );
    assert_format(
        "#[page(POST\"/login\")]\nfn login() {}",
        "#[page(POST \"/login\")]\nfn login() {}",
    );
}

#[test]
fn formats_a_method_list() {
    assert_format(
        "#[route([GET,POST]   \"/items\")]\nfn items() {}",
        "#[route([GET, POST] \"/items\")]\nfn items() {}",
    );
}

#[test]
fn formats_context_entries_as_expressions() {
    assert_format(
        "#[page(\"/\", with( Head::new().title( \"Home\" ) ))]\nfn home() {}",
        "#[page(\"/\", with(Head::new().title(\"Home\")))]\nfn home() {}",
    );
}

#[test]
fn puts_each_argument_on_its_own_line_when_too_long() {
    assert_format(
        "#[layout(\"/docs\", with(Head::new().title(\"Documentation of every single feature\")), with(Sidebar::default()))]\nfn docs() {}",
        "#[layout(\n    \"/docs\",\n    with(Head::new().title(\"Documentation of every single feature\")),\n    with(Sidebar::default())\n)]\nfn docs() {}",
    );
}

#[test]
fn drops_a_comma_after_the_last_argument() {
    assert_format(
        "#[route(GET \"/\", with(Head::new().title(\"Home\")),)]\nfn home() {}",
        "#[route(GET \"/\", with(Head::new().title(\"Home\")))]\nfn home() {}",
    );
}

#[test]
fn formats_an_error_option() {
    assert_format(
        "#[query_params(error   =   bad_request)]\nstruct Search;",
        "#[query_params(error = bad_request)]\nstruct Search;",
    );
    assert_format(
        "#[query_params(error = redirect( \"/login\" ))]\nstruct Search;",
        "#[query_params(error = redirect(\"/login\"))]\nstruct Search;",
    );
}

#[test]
fn formats_a_path_parameter() {
    assert_format(
        "path_param!(pub   *post_ids :  u64   else   bad_request(\"invalid\"),);",
        "path_param!(pub *post_ids: u64 else bad_request(\"invalid\"));",
    );
    assert_format(
        "module_param!(pub(crate)   post_id:u64 else not_found);",
        "module_param!(pub(crate) post_id: u64 else not_found);",
    );
}

#[test]
fn formats_segment_options() {
    assert_format(
        "segment!(kind:CatchAll,rename:\"rest\");",
        "segment!(kind: CatchAll, rename: \"rest\");",
    );
}

#[test]
fn formats_a_braced_not_found_prefix() {
    assert_format("not_found! {   \"/docs\"   }", "not_found! { \"/docs\" }");
}

#[test]
fn keeps_comments_between_arguments() {
    let expected = "#[page(\n    \"/\",\n    // The title shown in the browser tab.\n    with(Head::new().title(\"Home\"))\n)]\nfn home() {}";
    assert_format(expected, expected);
}

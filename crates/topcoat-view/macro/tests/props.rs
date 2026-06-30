use topcoat::view::Props;

#[derive(Props, Debug, PartialEq)]
struct ButtonProps {
    #[into]
    label: String,
    kind: u8,
    #[default]
    disabled: bool,
}

#[test]
fn builder_sets_required_and_default_fields() {
    let props = ButtonProps::builder()
        .label("Save")
        .kind(1)
        .disabled(true)
        .build();

    assert_eq!(
        props,
        ButtonProps {
            label: "Save".to_owned(),
            kind: 1,
            disabled: true,
        }
    );
}

#[test]
fn unset_default_field_uses_default_value() {
    let props = ButtonProps::builder().label("Save").kind(1).build();

    assert!(!props.disabled);
}

#[test]
fn setters_can_be_called_more_than_once() {
    let props = ButtonProps::builder()
        .label("Save")
        .kind(1)
        .kind(2)
        .label("Submit")
        .build();

    assert_eq!(props.label, "Submit");
    assert_eq!(props.kind, 2);
}

#[test]
fn builder_is_reachable_through_the_props_trait() {
    fn build<P: Props>() -> P::Builder {
        P::builder()
    }

    let props = build::<ButtonProps>().label("Save").kind(1).build();

    assert_eq!(props.label, "Save");
}

#[derive(Props, Debug, PartialEq)]
struct ListProps<T: Clone>
where
    T: std::fmt::Debug,
{
    items: Vec<T>,
    #[into]
    title: String,
    #[default]
    #[into]
    empty_message: String,
}

#[test]
fn generic_struct_builds_with_bounds_and_where_clause() {
    let props = ListProps::builder()
        .items(vec![1, 2, 3])
        .title("Numbers")
        .build();

    assert_eq!(
        props,
        ListProps {
            items: vec![1, 2, 3],
            title: "Numbers".to_owned(),
            empty_message: String::new(),
        }
    );
}

#[test]
fn default_and_into_combine_on_one_field() {
    let props = ListProps::<u8>::builder()
        .items(vec![])
        .title("Empty")
        .empty_message("Nothing here")
        .build();

    assert_eq!(props.empty_message, "Nothing here");
}

#[derive(Props)]
struct LifetimeProps<'a, T> {
    value: &'a T,
    #[default]
    count: usize,
}

#[test]
fn lifetimes_carry_over_to_the_builder() {
    let value = "hello".to_owned();
    let props = LifetimeProps::builder().value(&value).build();

    assert_eq!(props.value, "hello");
    assert_eq!(props.count, 0);
}

#[derive(Props)]
struct AllDefaultProps {
    #[default]
    a: u32,
    #[default]
    b: String,
}

#[test]
fn struct_with_only_default_fields_builds_immediately() {
    let props = AllDefaultProps::builder().build();

    assert_eq!(props.a, 0);
    assert_eq!(props.b, "");

    let props = AllDefaultProps::builder().a(7).build();

    assert_eq!(props.a, 7);
}

#[derive(Props, Debug, PartialEq)]
struct DefaultExprProps {
    #[default(5)]
    limit: u32,
    #[default("page".to_owned())]
    #[into]
    label: String,
}

#[test]
fn unset_default_expr_field_uses_the_expression() {
    let props = DefaultExprProps::builder().build();

    assert_eq!(
        props,
        DefaultExprProps {
            limit: 5,
            label: "page".to_owned(),
        }
    );
}

#[test]
fn set_default_expr_field_overrides_the_expression() {
    let props = DefaultExprProps::builder()
        .limit(10)
        .label("section")
        .build();

    assert_eq!(props.limit, 10);
    assert_eq!(props.label, "section");
}

#[derive(Props)]
struct KeywordProps {
    r#type: String,
    #[default]
    r#for: String,
}

#[test]
fn raw_identifier_fields_get_setters() {
    let props = KeywordProps::builder()
        .r#type("button".to_owned())
        .r#for("email".to_owned())
        .build();

    assert_eq!(props.r#type, "button");
    assert_eq!(props.r#for, "email");
}

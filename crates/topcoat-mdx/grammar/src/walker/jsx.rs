//! JSX component walking and HTML element overrides.
//!
//! Handles MDX JSX elements (`MdxJsxFlowElement`, `MdxJsxTextElement`),
//! attribute coercion, and the override mechanism that lets registered
//! components replace standard HTML elements.

use proc_macro2::Span;
use syn::{
    Expr, LitStr, Path as SynPath,
    token::{Colon, Paren},
};
use topcoat_view_grammar::{
    attributes::{AttributeKey, AttributeNode, AttributeValue, Attributes},
    view::{Component, NamedArg, NamedArgValue, Node, Nodes},
};

use super::{WalkContext, helpers::make_ident};

// ---------------------------------------------------------------------------
// JSX component walking
// ---------------------------------------------------------------------------

/// Smart-coerce an MDX attribute string to a typed Rust literal.
///
/// `"true"` / `"false"` -> bool, pure integers -> `LitInt`,
/// pure floats -> `LitFloat`, everything else -> `LitStr`.
/// Leading-zero digit strings (e.g. `"007"`) stay as strings because
/// `syn::LitInt` rejects them.
///
/// The `span` argument is used for the fallback `LitStr` so that
/// compiler diagnostics point to the `compile_mdx!` invocation rather
/// than `call_site()`.
#[must_use]
pub fn coerce_attr_value(value: &str, span: Span) -> Expr {
    match value {
        "true" => return syn::parse_quote!(true),
        "false" => return syn::parse_quote!(false),
        _ => {}
    }
    // Try integer, but reject leading-zero strings like "007" (syn 2.0
    // accepts them as valid LitInt values, so we guard manually).
    if !(value.len() > 1 && value.starts_with('0'))
        && let Ok(lit) = syn::parse_str::<syn::LitInt>(value)
    {
        return Expr::Lit(syn::ExprLit {
            attrs: vec![],
            lit: syn::Lit::Int(lit),
        });
    }
    // Try float.
    if let Ok(lit) = syn::parse_str::<syn::LitFloat>(value) {
        return Expr::Lit(syn::ExprLit {
            attrs: vec![],
            lit: syn::Lit::Float(lit),
        });
    }
    // Default: string literal.
    Expr::Lit(syn::ExprLit {
        attrs: vec![],
        lit: syn::Lit::Str(LitStr::new(value, span)),
    })
}

/// Tries to apply an HTML element override from the `WalkContext`.
///
/// Production code uses `try_find_override_path` + `build_override_component`
/// to avoid consuming children when no override is registered. This function
/// exists solely for unit tests that need a single-call variant, and delegates
/// to the production builder so that both paths behave identically.
#[cfg(test)]
pub(crate) fn try_apply_override(
    ctx: &WalkContext,
    tag: &str,
    attributes: &Attributes,
    children: Nodes,
) -> Option<Node> {
    let path = try_find_override_path(ctx, tag)?;
    Some(build_override_component(
        path, attributes, children, ctx.span,
    ))
}

/// Returns the override path for the given tag without consuming children.
///
/// Unlike `try_apply_override`, this only performs the lookup and returns
/// the component path, so it can be used as a guard before deciding whether
/// to pass owned values into the component builder.
#[inline]
pub(crate) fn try_find_override_path(ctx: &WalkContext, tag: &str) -> Option<SynPath> {
    ctx.overrides
        .iter()
        .find_map(|(t, p)| if *t == tag { Some(p.clone()) } else { None })
}

/// Builds a `Node::Component` from a pre-resolved override path.
///
/// Use `try_find_override_path` to check for an override first, then pass
/// the path here along with the owned children. This avoids the ownership
/// issue where `try_apply_override` would consume `children` even when
/// no override was registered.
pub(crate) fn build_override_component(
    path: SynPath,
    attributes: &Attributes,
    children: Nodes,
    span: Span,
) -> Node {
    let named_args: Vec<NamedArg> = attributes
        .items
        .iter()
        .filter_map(|attr_node| {
            if let AttributeNode::Attribute(attr) = attr_node
                && let AttributeKey::Ident(ident) = &attr.key
            {
                let name = html_ident_to_string(ident);
                let expr: Expr = match &attr.value {
                    AttributeValue::LitStr(s) => coerce_attr_value(&s.value(), span),
                    AttributeValue::Expr(_) => syn::parse_quote!(true),
                };
                return Some(NamedArg {
                    ident: make_ident(&name),
                    colon: Colon::default(),
                    value: NamedArgValue::Expr(expr),
                });
            }
            None
        })
        .collect();

    Node::Component(Component {
        path,
        paren_token: Paren::default(),
        named_args,
        children,
    })
}

/// Converts an `HtmlIdent` (which may contain hyphen/colon/dot segments) into
/// a valid Rust identifier name. Hyphenated segments like `data-lang` become
/// `snake_case` (`data_lang`). Colon and dot separators are converted to
/// underscores. Used by `build_override_component` to forward attribute names
/// to the override component's props builder.
fn html_ident_to_string(ident: &topcoat_view_grammar::view::HtmlIdent) -> String {
    let mut result = ident.first.to_string();
    for seg in &ident.rest {
        let sep_char = match &seg.separator {
            topcoat_view_grammar::view::HtmlIdentSeparator::Dash(_)
            | topcoat_view_grammar::view::HtmlIdentSeparator::Colon(_)
            | topcoat_view_grammar::view::HtmlIdentSeparator::Dot(_) => '_',
        };
        result.push(sep_char);
        match &seg.part {
            topcoat_view_grammar::view::HtmlIdentPart::Ident(i) => result.push_str(&i.to_string()),
            topcoat_view_grammar::view::HtmlIdentPart::Int(lit) => {
                result.push_str(lit.base10_digits());
            }
        }
    }
    result
}

/// Walk JSX attributes from an mdast element into `NamedArg`s.
///
/// - Bare attributes (value: None) -> `true`.
/// - Literal attributes -> `coerce_attr_value`.
/// - Expression attributes (`{...spread}`) -> skipped.
/// - Namespaced attribute names -> pushed to `ctx.errors`.
pub(crate) fn walk_jsx_attributes(
    ctx: &WalkContext,
    attrs: &[markdown::mdast::AttributeContent],
) -> Vec<NamedArg> {
    attrs
        .iter()
        .filter_map(|content| match content {
            markdown::mdast::AttributeContent::Property(attr) => {
                // Skip namespaced attribute names (e.g. xml:lang).
                if attr.name.contains(':') {
                    ctx.errors.borrow_mut().push(format!(
                        "namespaced attribute '{}' not supported",
                        attr.name
                    ));
                    return None;
                }
                let value = match &attr.value {
                    Some(markdown::mdast::AttributeValue::Literal(s)) => {
                        coerce_attr_value(s, ctx.span)
                    }
                    None => syn::parse_quote!(true), // bare attribute becomes true
                    Some(markdown::mdast::AttributeValue::Expression(_)) => {
                        // Expression attributes like `{value}` are out of scope.
                        ctx.errors.borrow_mut().push(format!(
                            "expression attribute '{}' not supported",
                            attr.name
                        ));
                        return None;
                    }
                };
                Some(NamedArg {
                    ident: make_ident(&attr.name),
                    colon: Colon::default(),
                    value: NamedArgValue::Expr(value),
                })
            }
            markdown::mdast::AttributeContent::Expression(_) => {
                // Spread attributes like `{...props}` are out of scope.
                ctx.errors
                    .borrow_mut()
                    .push("spread attributes not supported".to_string());
                None
            }
        })
        .collect()
}

/// Walk a flow-level JSX element (`MdxJsxFlowElement`) into `Option<Node::Component>`.
///
/// Returns `Some` when the element name is `PascalCase` and registered in
/// `ctx.components`. Returns `None` for:
/// - Lowercase names (HTML elements, not components).
/// - Fragments (`name: None`).
/// - Unregistered `PascalCase` names (pushes error to `ctx.errors`).
pub fn walk_jsx_element(
    ctx: &WalkContext,
    element: &markdown::mdast::MdxJsxFlowElement,
) -> Option<Node> {
    let name = element.name.as_deref()?;
    // Fragments (name: None) handled by the `?` above.

    // Lowercase = HTML element, not a component.
    if !name.starts_with(char::is_uppercase) {
        return None;
    }

    // Look up in component registry.
    let path = if let Some((_, p)) = ctx.components.iter().find(|(tag, _)| tag == name) {
        p.clone()
    } else {
        ctx.errors
            .borrow_mut()
            .push(format!("unknown component '{name}'"));
        return None;
    };

    let named_args = walk_jsx_attributes(ctx, &element.attributes);
    let children = super::walk_nodes(ctx, &element.children);

    Some(Node::Component(Component {
        path,
        paren_token: Paren::default(),
        named_args,
        children,
    }))
}

/// Walk a text-level JSX element (`MdxJsxTextElement`) into `Option<Node::Component>`.
///
/// Same logic as `walk_jsx_element` but for inline JSX (e.g. `<Inline>`
/// inside a paragraph). Pushes an error to `ctx.errors` for unregistered
/// `PascalCase` components so the author gets a compile-time diagnostic.
pub fn walk_jsx_text_element(
    ctx: &WalkContext,
    element: &markdown::mdast::MdxJsxTextElement,
) -> Option<Node> {
    let name = element.name.as_deref()?;

    if !name.starts_with(char::is_uppercase) {
        return None;
    }

    let path = if let Some((_, p)) = ctx.components.iter().find(|(tag, _)| tag == name) {
        p.clone()
    } else {
        ctx.errors
            .borrow_mut()
            .push(format!("unknown component '{name}'"));
        return None;
    };

    let named_args = walk_jsx_attributes(ctx, &element.attributes);
    let children = super::walk_nodes(ctx, &element.children);

    Some(Node::Component(Component {
        path,
        paren_token: Paren::default(),
        named_args,
        children,
    }))
}

/// Builds a normal (non-void) element for `tag`, or the registered override
/// component if one exists. Centralizes the find-and-branch that was
/// previously duplicated at each element-construction call site.
pub(crate) fn element_or_override(
    ctx: &WalkContext,
    tag: &str,
    attributes: Attributes,
    children: Nodes,
) -> Node {
    if let Some(path) = try_find_override_path(ctx, tag) {
        build_override_component(path, &attributes, children, ctx.span)
    } else {
        Node::Element(Box::new(super::helpers::normal_element_with_attrs(
            tag, attributes, children,
        )))
    }
}

/// Same as `element_or_override`, for void elements (no children).
pub(crate) fn void_element_or_override(
    ctx: &WalkContext,
    tag: &str,
    attributes: Attributes,
) -> Node {
    if let Some(path) = try_find_override_path(ctx, tag) {
        build_override_component(path, &attributes, Nodes::new(), ctx.span)
    } else {
        Node::Element(Box::new(super::helpers::void_element_with_attrs(
            tag, attributes,
        )))
    }
}

#[cfg(test)]
mod tests {
    use syn::{Lit, LitBool, Path};

    use super::*;
    use crate::parse::get_parse_options;

    fn parse_and_walk_ctx(ctx: &WalkContext, content: &str) -> Nodes {
        let options = get_parse_options();
        let root = markdown::to_mdast(content, &options).unwrap();
        match root {
            markdown::mdast::Node::Root(r) => super::super::walk_nodes(ctx, &r.children),
            _ => unreachable!(),
        }
    }

    // ---- coerce_attr_value tests ----

    #[test]
    fn coerce_attr_value_bool_true() {
        let expr = coerce_attr_value("true", Span::call_site());
        assert!(matches!(
            expr,
            Expr::Lit(syn::ExprLit {
                lit: Lit::Bool(LitBool { value: true, .. }),
                ..
            })
        ));
    }

    #[test]
    fn coerce_attr_value_bool_false() {
        let expr = coerce_attr_value("false", Span::call_site());
        assert!(matches!(
            expr,
            Expr::Lit(syn::ExprLit {
                lit: Lit::Bool(LitBool { value: false, .. }),
                ..
            })
        ));
    }

    #[test]
    fn coerce_attr_value_int() {
        let expr = coerce_attr_value("42", Span::call_site());
        assert!(
            matches!(
                expr,
                Expr::Lit(syn::ExprLit {
                    lit: Lit::Int(_),
                    ..
                })
            ),
            "expected LitInt"
        );
    }

    #[test]
    fn coerce_attr_value_float() {
        let expr = coerce_attr_value("3.14", Span::call_site());
        assert!(
            matches!(
                expr,
                Expr::Lit(syn::ExprLit {
                    lit: Lit::Float(_),
                    ..
                })
            ),
            "expected LitFloat"
        );
    }

    #[test]
    fn coerce_attr_value_string() {
        let expr = coerce_attr_value("hello", Span::call_site());
        if let Expr::Lit(l) = expr {
            assert!(
                matches!(l.lit, Lit::Str(s) if s.value() == "hello"),
                "expected LitStr(\"hello\")"
            );
        } else {
            panic!("expected Expr::Lit");
        }
    }

    #[test]
    fn coerce_attr_value_empty_string_stays_str() {
        let expr = coerce_attr_value("", Span::call_site());
        assert!(
            matches!(expr, Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) if s.value().is_empty()),
            "empty string should coerce to LitStr(\"\")"
        );
    }

    #[test]
    fn coerce_attr_value_leading_zeros_stay_str() {
        let expr = coerce_attr_value("007", Span::call_site());
        assert!(
            matches!(expr, Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) if s.value() == "007"),
            "leading zeros should stay as string, not coerce to int"
        );
    }

    // ---- JSX attribute walking tests ----

    #[test]
    fn walk_jsx_attributes_bare() {
        let ctx = WalkContext::empty();
        let attr = markdown::mdast::MdxJsxAttribute {
            name: "disabled".to_string(),
            value: None,
        };
        let attrs = vec![markdown::mdast::AttributeContent::Property(attr)];
        let result = walk_jsx_attributes(&ctx, &attrs);
        assert_eq!(result.len(), 1);
        let named_arg = &result[0];
        assert_eq!(named_arg.ident.to_string(), "disabled");
        assert!(
            matches!(
                &named_arg.value,
                NamedArgValue::Expr(Expr::Lit(syn::ExprLit {
                    lit: Lit::Bool(LitBool { value: true, .. }),
                    ..
                }))
            ),
            "bare attribute should coerce to true"
        );
    }

    #[test]
    fn walk_jsx_attributes_key_value() {
        let ctx = WalkContext::empty();
        let attr = markdown::mdast::MdxJsxAttribute {
            name: "label".to_string(),
            value: Some(markdown::mdast::AttributeValue::Literal(
                "hello".to_string(),
            )),
        };
        let attrs = vec![markdown::mdast::AttributeContent::Property(attr)];
        let result = walk_jsx_attributes(&ctx, &attrs);
        assert_eq!(result.len(), 1);
        let named_arg = &result[0];
        assert_eq!(named_arg.ident.to_string(), "label");
        assert!(
            matches!(&named_arg.value, NamedArgValue::Expr(Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. })) if s.value() == "hello"),
            "string attribute value should coerce to LitStr"
        );
    }

    #[test]
    fn walk_jsx_attributes_skip_expression() {
        let ctx = WalkContext::new(&[], &[], Span::call_site());
        let expr_attr = markdown::mdast::AttributeContent::Expression(
            markdown::mdast::MdxJsxExpressionAttribute {
                value: "...props".to_string(),
                stops: vec![],
            },
        );
        let result = walk_jsx_attributes(&ctx, &[expr_attr]);
        assert!(result.is_empty(), "spread attributes should be skipped");
        let errors = ctx.errors.borrow();
        assert!(
            !errors.is_empty(),
            "should push error for spread attributes"
        );
    }

    // ---- JSX element walking tests ----

    #[test]
    fn walk_jsx_element_unknown_component() {
        let ctx = WalkContext::new(&[], &[], Span::call_site());
        let element = markdown::mdast::MdxJsxFlowElement {
            children: vec![],
            position: None,
            name: Some("Unknown".to_string()),
            attributes: vec![],
        };
        let result = walk_jsx_element(&ctx, &element);
        assert!(
            result.is_none(),
            "unregistered component should return None"
        );
        let errors = ctx.errors.borrow();
        assert!(
            !errors.is_empty(),
            "should push error for unknown component"
        );
    }

    #[test]
    fn walk_jsx_element_lowercase_is_html() {
        let ctx = WalkContext::new(&[], &[], Span::call_site());
        let element = markdown::mdast::MdxJsxFlowElement {
            children: vec![],
            position: None,
            name: Some("div".to_string()),
            attributes: vec![],
        };
        let result = walk_jsx_element(&ctx, &element);
        assert!(
            result.is_none(),
            "lowercase JSX should return None (HTML, not component)"
        );
        let errors = ctx.errors.borrow();
        assert!(errors.is_empty(), "lowercase should NOT push an error");
    }

    #[test]
    fn walk_jsx_element_fragment() {
        let ctx = WalkContext::new(&[], &[], Span::call_site());
        let element = markdown::mdast::MdxJsxFlowElement {
            children: vec![],
            position: None,
            name: None, // fragment
            attributes: vec![],
        };
        let result = walk_jsx_element(&ctx, &element);
        assert!(result.is_none(), "fragment should return None");
    }

    #[test]
    fn walk_jsx_element_registered_produces_component() {
        let component_path: Path = syn::parse_quote!(components::callout);
        let registry = vec![("Callout".to_string(), component_path)];
        let ctx = WalkContext::new(&registry, &[], Span::call_site());
        let element = markdown::mdast::MdxJsxFlowElement {
            children: vec![],
            position: None,
            name: Some("Callout".to_string()),
            attributes: vec![],
        };
        let result = walk_jsx_element(&ctx, &element);
        assert!(
            matches!(&result, Some(Node::Component(_))),
            "registered component should produce Node::Component"
        );
        if let Some(Node::Component(comp)) = result {
            assert_eq!(
                comp.path.segments.last().unwrap().ident.to_string(),
                "callout"
            );
            assert!(comp.named_args.is_empty());
            assert!(comp.children.is_empty());
        }
    }

    #[test]
    fn walk_jsx_element_self_closing_empty_children() {
        let component_path: Path = syn::parse_quote!(components::divider);
        let registry = vec![("Divider".to_string(), component_path)];
        let ctx = WalkContext::new(&registry, &[], Span::call_site());
        // Self-closing and closed tags both produce empty children in markdown-rs.
        let element = markdown::mdast::MdxJsxFlowElement {
            children: vec![], // empty
            position: None,
            name: Some("Divider".to_string()),
            attributes: vec![],
        };
        let result = walk_jsx_element(&ctx, &element);
        assert!(
            matches!(&result, Some(Node::Component(_))),
            "self-closing component should produce Node::Component"
        );
        if let Some(Node::Component(comp)) = result {
            assert!(
                comp.children.is_empty(),
                "self-closing should have empty children"
            );
        }
    }

    // ---- walk_node JSX dispatch tests ----

    #[test]
    fn walk_node_dispatches_mdx_jsx_flow_element() {
        // With html_flow disabled, markdown-rs parses <Widget></Widget> as
        // MdxJsxFlowElement. Self-closing <Widget /> also works correctly.
        let component_path: Path = syn::parse_quote!(my::widget);
        let registry = vec![("Widget".to_string(), component_path)];
        let ctx = WalkContext::new(&registry, &[], Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "<Widget></Widget>");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "walk_node should dispatch MdxJsxFlowElement to walk_jsx_element"
        );
    }

    #[test]
    fn walk_jsx_text_element_registered_produces_component() {
        // Walk a manually-constructed MdxJsxTextElement through walk_jsx_text_element.
        // Note: markdown-rs only produces MdxJsxTextElement in specific parsing contexts;
        // this test verifies the walker function works on the struct directly.
        let component_path: Path = syn::parse_quote!(inline::badge);
        let registry = vec![("Badge".to_string(), component_path)];
        let ctx = WalkContext::new(&registry, &[], Span::call_site());
        let element = markdown::mdast::MdxJsxTextElement {
            children: vec![],
            position: None,
            name: Some("Badge".to_string()),
            attributes: vec![],
        };
        let result = walk_jsx_text_element(&ctx, &element);
        assert!(
            matches!(&result, Some(Node::Component(_))),
            "registered text JSX should produce Node::Component"
        );
    }

    #[test]
    fn walk_jsx_text_element_unknown_component_pushes_error() {
        // Inline text-level components must report errors for unknown
        // PascalCase names, matching flow-level walk_jsx_element behavior.
        let ctx = WalkContext::new(&[], &[], Span::call_site());
        let element = markdown::mdast::MdxJsxTextElement {
            children: vec![],
            position: None,
            name: Some("Unknown".to_string()),
            attributes: vec![],
        };
        let result = walk_jsx_text_element(&ctx, &element);
        assert!(
            result.is_none(),
            "unregistered inline component should return None"
        );
        let errors = ctx.errors.borrow();
        assert!(
            !errors.is_empty(),
            "should push error for unknown inline component"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("unknown component 'Unknown'")),
            "should contain 'unknown component' message. Errors: {:?}",
            *errors
        );
    }

    // ---- override mechanism tests ----

    #[test]
    fn try_apply_override_hits() {
        let component_path: Path = syn::parse_quote!(components::custom_link);
        let leaked: &'static str = String::leak("a".to_string());
        let overrides: [(&'static str, Path); 1] =
            [(leaked as &'static str, component_path.clone())];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let attrs =
            super::super::helpers::with_attributes(vec![super::super::helpers::create_attribute(
                "href",
                "https://example.com",
            )]);
        let children = Nodes::from(vec![super::super::helpers::text_node("click")]);
        let result = try_apply_override(&ctx, "a", &attrs, children);
        assert!(result.is_some(), "should return Some when tag has override");
        if let Some(Node::Component(comp)) = result {
            assert_eq!(
                comp.path.segments.last().unwrap().ident.to_string(),
                "custom_link"
            );
            assert_eq!(comp.named_args.len(), 1);
            assert_eq!(comp.named_args[0].ident.to_string(), "href");
        } else {
            panic!("expected Node::Component");
        }
    }

    #[test]
    fn try_apply_override_misses() {
        let ctx = WalkContext::empty_with_overrides(&[]);
        let attrs =
            super::super::helpers::with_attributes(vec![super::super::helpers::create_attribute(
                "src",
                "photo.png",
            )]);
        let children = Nodes::new();
        let result = try_apply_override(&ctx, "img", &attrs, children);
        assert!(
            result.is_none(),
            "should return None when tag has no override"
        );
    }

    #[test]
    fn walk_link_with_override() {
        let component_path: Path = syn::parse_quote!(components::custom_link);
        let leaked: &'static str = String::leak("a".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "[link](https://example.com)");
        // Should be inside a paragraph
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            // The link should render as a Component, not an <a> element.
            let has_component = p.children().iter().any(|c| matches!(c, Node::Component(_)));
            let has_a = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("a")
                } else {
                    false
                }
            });
            assert!(
                has_component,
                "paragraph should contain Component for overridden link"
            );
            assert!(
                !has_a,
                "paragraph should NOT contain <a> when override is registered"
            );
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn walk_link_override_blocks_xss() {
        let component_path: Path = syn::parse_quote!(components::custom_link);
        let leaked: &'static str = String::leak("a".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "[xss](javascript:alert(1))");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            // Dangerous URL should produce <span>, NOT a Component.
            let has_component = p.children().iter().any(|c| matches!(c, Node::Component(_)));
            let has_span = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("span")
                } else {
                    false
                }
            });
            assert!(
                !has_component,
                "javascript: link should NOT produce Component even when override is registered"
            );
            assert!(
                has_span,
                "javascript: link should produce <span> for XSS protection"
            );
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn element_or_override_hits() {
        let component_path: Path = syn::parse_quote!(components::custom_p);
        let leaked: &'static str = String::leak("p".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let attrs = Attributes::default();
        let children = Nodes::from(vec![super::super::helpers::text_node("hi")]);
        let result = element_or_override(&ctx, "p", attrs, children);
        assert!(
            matches!(result, Node::Component(_)),
            "registered tag should produce Node::Component"
        );
    }

    #[test]
    fn element_or_override_misses() {
        let ctx = WalkContext::empty();
        let attrs = Attributes::default();
        let children = Nodes::from(vec![super::super::helpers::text_node("hi")]);
        let result = element_or_override(&ctx, "p", attrs, children);
        assert!(
            matches!(result, Node::Element(e) if e.name().string_name().as_deref() == Some("p")),
            "unregistered tag should produce plain <p> element"
        );
    }

    #[test]
    fn void_element_or_override_hits() {
        let component_path: Path = syn::parse_quote!(components::custom_hr);
        let leaked: &'static str = String::leak("hr".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let result = void_element_or_override(&ctx, "hr", Attributes::default());
        assert!(
            matches!(result, Node::Component(_)),
            "registered void tag should produce Node::Component"
        );
    }

    #[test]
    fn void_element_or_override_misses() {
        let ctx = WalkContext::empty();
        let result = void_element_or_override(&ctx, "hr", Attributes::default());
        assert!(
            matches!(result, Node::Element(e) if e.name().string_name().as_deref() == Some("hr")),
            "unregistered void tag should produce plain <hr> element"
        );
    }

    #[test]
    fn walk_paragraph_with_p_override() {
        let component_path: Path = syn::parse_quote!(components::paragraph);
        let leaked: &'static str = String::leak("p".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "Plain text");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "p override should produce Node::Component"
        );
    }

    #[test]
    fn walk_paragraph_without_override_falls_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "Plain text");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Element(e) if e.name().string_name().as_deref() == Some("p")),
            "paragraph without override should produce <p>"
        );
    }

    #[test]
    fn walk_strong_with_override() {
        let component_path: Path = syn::parse_quote!(components::bold);
        let leaked: &'static str = String::leak("strong".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "**bold**");
        if let Node::Element(p) = &nodes[0] {
            let has_component = p.children().iter().any(|c| matches!(c, Node::Component(_)));
            assert!(has_component, "strong override should produce Component");
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn walk_strong_without_override_falls_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "**bold**");
        if let Node::Element(p) = &nodes[0] {
            let has_strong = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("strong")
                } else {
                    false
                }
            });
            assert!(
                has_strong,
                "strong without override should produce <strong>"
            );
        }
    }

    #[test]
    fn walk_blockquote_with_override() {
        let component_path: Path = syn::parse_quote!(components::quote);
        let leaked: &'static str = String::leak("blockquote".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "> quoted");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "blockquote override should produce Node::Component"
        );
    }

    #[test]
    fn walk_blockquote_without_override_falls_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "> quoted");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Element(e) if e.name().string_name().as_deref() == Some("blockquote")),
            "blockquote without override should produce <blockquote>"
        );
    }

    #[test]
    fn walk_inline_code_with_override() {
        let component_path: Path = syn::parse_quote!(components::inline_code);
        let leaked: &'static str = String::leak("code".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "`inline`");
        if let Node::Element(p) = &nodes[0] {
            let has_component = p.children().iter().any(|c| matches!(c, Node::Component(_)));
            assert!(
                has_component,
                "inline code override should produce Component"
            );
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn walk_inline_code_without_override_falls_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "`inline`");
        if let Node::Element(p) = &nodes[0] {
            let has_code = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("code")
                } else {
                    false
                }
            });
            assert!(
                has_code,
                "inline code without override should produce <code>"
            );
        }
    }

    #[test]
    fn walk_unordered_list_with_ul_override() {
        let component_path: Path = syn::parse_quote!(components::list);
        let leaked: &'static str = String::leak("ul".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "- item");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "ul override should produce Node::Component"
        );
    }

    #[test]
    fn walk_ordered_list_with_ol_override() {
        let component_path: Path = syn::parse_quote!(components::list);
        let leaked: &'static str = String::leak("ol".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "1. item");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "ol override should produce Node::Component"
        );
    }

    #[test]
    fn walk_list_without_override_falls_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "- item");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Element(e) if e.name().string_name().as_deref() == Some("ul")),
            "list without override should produce <ul>"
        );
    }

    #[test]
    fn walk_list_item_with_li_override() {
        let component_path: Path = syn::parse_quote!(components::item);
        let leaked: &'static str = String::leak("li".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "- item");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(ul) = &nodes[0] {
            let has_component = ul
                .children()
                .iter()
                .any(|c| matches!(c, Node::Component(_)));
            assert!(
                has_component,
                "li override should produce Component inside <ul>"
            );
        } else {
            panic!("expected <ul> element (only li is overridden, not ul)");
        }
    }

    #[test]
    fn walk_list_item_without_override_falls_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "- item");
        if let Node::Element(ul) = &nodes[0] {
            let has_li = ul.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("li")
                } else {
                    false
                }
            });
            assert!(has_li, "list item without override should produce <li>");
        }
    }

    #[test]
    fn walk_table_with_override() {
        let component_path: Path = syn::parse_quote!(components::data_table);
        let leaked: &'static str = String::leak("table".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "| A |\n|---|\n| 1 |");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "table override should produce Node::Component"
        );
    }

    #[test]
    fn walk_table_without_override_falls_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "| A |\n|---|\n| 1 |");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Element(e) if e.name().string_name().as_deref() == Some("table")),
            "table without override should produce <table>"
        );
    }

    fn find_element_recursive<'a>(
        element: &'a topcoat_view_grammar::view::Element,
        tag: &str,
    ) -> Option<&'a topcoat_view_grammar::view::Element> {
        if element.name().string_name().as_deref() == Some(tag) {
            return Some(element);
        }
        for child in element.children() {
            if let Node::Element(inner) = child
                && let Some(found) = find_element_recursive(inner, tag)
            {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn walk_table_cell_with_th_override() {
        let component_path: Path = syn::parse_quote!(components::header_cell);
        let leaked: &'static str = String::leak("th".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "| A |\n|---|\n| 1 |");
        if let Node::Element(table) = &nodes[0] {
            let thead = table
                .children()
                .iter()
                .find_map(|c| {
                    if let Node::Element(e) = c {
                        (e.name().string_name().as_deref() == Some("thead")).then_some(e.as_ref())
                    } else {
                        None
                    }
                })
                .expect("table should have thead");
            let tr = thead
                .children()
                .iter()
                .find_map(|c| {
                    if let Node::Element(e) = c {
                        (e.name().string_name().as_deref() == Some("tr")).then_some(e.as_ref())
                    } else {
                        None
                    }
                })
                .expect("thead should have tr");
            let has_component = tr
                .children()
                .iter()
                .any(|c| matches!(c, Node::Component(_)));
            assert!(
                has_component,
                "th override should produce Component inside <tr>"
            );
        } else {
            panic!("expected <table> element (only th is overridden, not table)");
        }
    }

    #[test]
    fn walk_table_cell_with_td_override() {
        let component_path: Path = syn::parse_quote!(components::data_cell);
        let leaked: &'static str = String::leak("td".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "| A |\n|---|\n| 1 |");
        if let Node::Element(table) = &nodes[0] {
            let tbody = table
                .children()
                .iter()
                .find_map(|c| {
                    if let Node::Element(e) = c {
                        (e.name().string_name().as_deref() == Some("tbody")).then_some(e.as_ref())
                    } else {
                        None
                    }
                })
                .expect("table should have tbody");
            let tr = tbody
                .children()
                .iter()
                .find_map(|c| {
                    if let Node::Element(e) = c {
                        (e.name().string_name().as_deref() == Some("tr")).then_some(e.as_ref())
                    } else {
                        None
                    }
                })
                .expect("tbody should have tr");
            let has_component = tr
                .children()
                .iter()
                .any(|c| matches!(c, Node::Component(_)));
            assert!(
                has_component,
                "td override should produce Component inside <tr>"
            );
        } else {
            panic!("expected <table> element (only td is overridden, not table)");
        }
    }

    #[test]
    fn walk_table_cells_without_override_fall_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "| A |\n|---|\n| 1 |");
        if let Node::Element(table) = &nodes[0] {
            let header_cell_found = find_element_recursive(table, "th").is_some();
            let data_cell_found = find_element_recursive(table, "td").is_some();
            assert!(
                header_cell_found,
                "table without override should still have <th>"
            );
            assert!(
                data_cell_found,
                "table without override should still have <td>"
            );
        }
    }

    #[test]
    fn walk_context_with_overrides() {
        let component_path: Path = syn::parse_quote!(components::custom_link);
        let leaked: &'static str = String::leak("a".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        assert_eq!(ctx.overrides.len(), 1);
        assert_eq!(ctx.overrides[0].0, "a");
    }

    // ---- expanded override tests ----

    #[test]
    fn walk_heading_with_h1_override() {
        let component_path: Path = syn::parse_quote!(components::heading);
        let leaked: &'static str = String::leak("h1".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "# Hello");
        assert_eq!(nodes.len(), 1);
        // With override, the heading should produce a Component, not an <h1> element.
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "h1 override should produce Node::Component"
        );
        if let Node::Component(comp) = &nodes[0] {
            assert_eq!(
                comp.path.segments.last().unwrap().ident.to_string(),
                "heading"
            );
        }
    }

    #[test]
    fn walk_heading_without_override_falls_through() {
        // No h1 override registered; should produce normal <h1>.
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "# Hello");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Element(e) if e.name().string_name().as_deref() == Some("h1")),
            "heading without override should produce <h1>"
        );
    }

    #[test]
    fn walk_image_with_override() {
        let component_path: Path = syn::parse_quote!(components::picture);
        let leaked: &'static str = String::leak("img".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "![alt text](photo.png)");
        // Should be inside a paragraph.
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            // The image should render as a Component, not an <img> void element.
            let has_component = p.children().iter().any(|c| matches!(c, Node::Component(_)));
            let has_img = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("img")
                } else {
                    false
                }
            });
            assert!(has_component, "image override should produce Component");
            assert!(!has_img, "image override should NOT produce <img>");
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn walk_image_without_override_falls_through() {
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "![alt](photo.png)");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            let has_img = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("img")
                } else {
                    false
                }
            });
            assert!(has_img, "image without override should produce <img>");
        }
    }

    #[test]
    fn walk_code_block_with_pre_override() {
        let component_path: Path = syn::parse_quote!(components::code_block);
        let leaked: &'static str = String::leak("pre".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "```rust\nfn main() {}\n```");
        assert_eq!(nodes.len(), 1);
        // With override, the code block should produce a Component, not <pre>.
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "pre override should produce Node::Component"
        );
        if let Node::Component(comp) = &nodes[0] {
            assert_eq!(
                comp.path.segments.last().unwrap().ident.to_string(),
                "code_block"
            );
        }
    }

    #[test]
    fn walk_thematic_break_with_hr_override() {
        let component_path: Path = syn::parse_quote!(components::separator);
        let leaked: &'static str = String::leak("hr".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked as &'static str, component_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        // Use "***" instead of "---" to avoid frontmatter ambiguity.
        let nodes = parse_and_walk_ctx(&ctx, "\n***\n");
        assert_eq!(nodes.len(), 1);
        // With override, the thematic break should produce a Component, not <hr>.
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "hr override should produce Node::Component"
        );
        if let Node::Component(comp) = &nodes[0] {
            assert_eq!(
                comp.path.segments.last().unwrap().ident.to_string(),
                "separator"
            );
        }
    }

    #[test]
    fn override_not_applied_when_tag_not_registered() {
        // Only "a" is registered; h1 should fall through to HTML.
        let link_path: Path = syn::parse_quote!(components::custom_link);
        let leaked_a: &'static str = String::leak("a".to_string());
        let overrides: [(&'static str, Path); 1] = [(leaked_a as &'static str, link_path)];
        let ctx = WalkContext::new(&[], &overrides, Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "# No override here");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Element(e) if e.name().string_name().as_deref() == Some("h1")),
            "unregistered tag should fall through to HTML element"
        );
    }

    // ---- self-closing JSX tag fix: tracer tests ----

    #[test]
    fn walk_self_closing_jsx_component() {
        // <Widget /> should be parsed as MdxJsxFlowElement (not raw Html)
        // and produce Node::Component when "Widget" is registered.
        let component_path: Path = syn::parse_quote!(my::widget);
        let registry = vec![("Widget".to_string(), component_path)];
        let ctx = WalkContext::new(&registry, &[], Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "<Widget />");
        assert_eq!(nodes.len(), 1, "self-closing JSX should produce one node");
        assert!(
            matches!(&nodes[0], Node::Component(_)),
            "<Widget /> should produce Node::Component but got non-Component node"
        );
        if let Node::Component(comp) = &nodes[0] {
            assert_eq!(
                comp.path.segments.last().unwrap().ident.to_string(),
                "widget"
            );
            assert!(
                comp.children.is_empty(),
                "self-closing should have no children"
            );
        }
        // Also verify no errors were pushed.
        assert!(
            ctx.errors.borrow().is_empty(),
            "should not push errors for registered self-closing component"
        );
    }

    #[test]
    fn walk_jsx_with_content_not_html() {
        // <Widget>text</Widget> with content is parsed as Paragraph > MdxJsxTextElement
        // (not as MdxJsxFlowElement or raw Html). The walker dispatches MdxJsxTextElement
        // to walk_jsx_text_element which produces Node::Component when registered.
        let component_path: Path = syn::parse_quote!(my::widget);
        let registry = vec![("Widget".to_string(), component_path)];
        let ctx = WalkContext::new(&registry, &[], Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, "<Widget>text</Widget>");
        assert_eq!(
            nodes.len(),
            1,
            "JSX with content should produce one root node (paragraph)"
        );
        // The root node is a <p> wrapping the Component.
        if let Node::Element(p) = &nodes[0] {
            assert_eq!(
                p.name().string_name().as_deref(),
                Some("p"),
                "root should be a paragraph wrapping the text-level JSX"
            );
            assert!(
                p.children().iter().any(|c| matches!(c, Node::Component(_))),
                "paragraph should contain Node::Component for registered Widget"
            );
        } else {
            panic!("expected paragraph element wrapping JSX text component");
        }
    }

    #[test]
    fn walk_raw_html_does_not_produce_element() {
        // After html_flow/html_text are disabled, raw HTML like <div>content</div>
        // is no longer parsed as Node::Html; it appears as text content.
        // The walker should NOT produce a <div> element from raw HTML.
        let ctx = WalkContext::empty();
        let nodes = parse_and_walk_ctx(&ctx, "<div>content</div>");
        // Verify no <div> Element is produced (raw HTML does not render as DOM).
        let has_div = nodes.iter().any(|n| {
            if let Node::Element(e) = n {
                e.name().string_name().as_deref() == Some("div")
            } else {
                false
            }
        });
        assert!(
            !has_div,
            "raw <div> should NOT produce an Element node after HTML disable"
        );
    }

    // ---- tracer integration test ----

    #[test]
    fn tracer_component_round_trip() {
        // End-to-end tracer: parse MDX with a component, walk it, assert
        // the Component node has the correct path, named_args, and children.
        //
        // With html_flow disabled, markdown-rs parses JSX tag pairs as
        // MdxJsxFlowElement (e.g. <Callout type="info"></Callout>).
        // Self-closing tags like <Callout /> also work correctly.
        let component_path: Path = syn::parse_quote!(components::callout);
        let registry = vec![("Callout".to_string(), component_path)];
        let ctx = WalkContext::new(&registry, &[], Span::call_site());
        let nodes = parse_and_walk_ctx(&ctx, r#"<Callout type="info"></Callout>"#);
        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];

        // The root-level JSX produces a Component node.
        assert!(
            matches!(node, Node::Component(_)),
            "expected Node::Component"
        );

        if let Node::Component(comp) = node {
            // Path matches registry entry.
            assert_eq!(
                comp.path.segments.last().unwrap().ident.to_string(),
                "callout"
            );

            // Has one NamedArg: type = "info" (string literal).
            assert_eq!(comp.named_args.len(), 1);
            assert_eq!(comp.named_args[0].ident.to_string(), "type");
            assert!(
                matches!(
                    &comp.named_args[0].value,
                    NamedArgValue::Expr(Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. })) if s.value() == "info"
                ),
                "type should be string literal \"info\""
            );

            // Empty children (self-closing / empty tag pair).
            assert!(
                comp.children.is_empty(),
                "empty tag pair should have no children"
            );
        }
    }
}

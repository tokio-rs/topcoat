use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, Pat};
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::hir::{
    Bindings, Scope,
    emit::{Emit, Emitter},
};

/// A `match` whose arm bodies are lowered into nested scopes.
pub(crate) struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

impl Emit for MatchExpr {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let expr = &self.expr;

        if !self.arms.iter().any(|arm| arm.body.is_async()) {
            // The taken arm builds its block right inside the arm, where
            // its pattern's bindings are alive.
            let arms = self.arms.iter().map(|arm| {
                let pat = &arm.pat;
                let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
                let body = arm.body.emit_block();
                quote! { #pat #guard => #body }
            });

            emitter.hoist(quote! {
                let #ident = match #expr {
                    #(#arms,)*
                };
            });
            emitter.burst(quote! { __b.view(#ident); });
            return;
        }

        // The arm bodies build views of different types; nested
        // `EitherView`s unify them, and only the taken arm is driven as this
        // position's unit. The arm's view takes its pattern's bindings with
        // it.
        let arm_count = self.arms.len();
        let arms = self.arms.iter().enumerate().map(|(index, arm)| {
            let pat = &arm.pat;
            let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
            let body = arm.body.emit_captured(&Bindings::of_pattern(pat));
            let body = nest_either(body, index, arm_count);
            quote! { #pat #guard => #body }
        });

        emitter.hoist(quote! {
            let #ident = match #expr {
                #(#arms,)*
            };
        });
        emitter.unit(Span::call_site(), &ident);
    }
}

/// Wraps one arm's view so all arms unify to the same `EitherView` type.
///
/// The arms are the leaves of a balanced tree of `EitherView`s, and arm
/// `index` is wrapped in the `left`s and `right`s on its path from the
/// root. The type then nests only logarithmically deep in the arm count.
fn nest_either(view: TokenStream, index: usize, arm_count: usize) -> TokenStream {
    if arm_count <= 1 {
        return view;
    }

    let split = arm_count / 2;
    if index < split {
        let view = nest_either(view, index, split);
        quote! { #topcoat_view::internal::EitherView::left(#view) }
    } else {
        let view = nest_either(view, index - split, arm_count - split);
        quote! { #topcoat_view::internal::EitherView::right(#view) }
    }
}

/// A single `pat (if guard)? => body` arm of a [`MatchExpr`].
pub(crate) struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Scope,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `left` (`false`) and `right` (`true`) steps from the root of the
    /// `EitherView` tree down to an arm's view.
    fn path(index: usize, arm_count: usize) -> Vec<bool> {
        nest_either(quote! { view }, index, arm_count)
            .to_string()
            .split_whitespace()
            .filter_map(|token| match token {
                "left" => Some(false),
                "right" => Some(true),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn arms_get_distinct_positions_in_a_shallow_either_tree() {
        for arm_count in 1..=40_usize {
            let paths = (0..arm_count)
                .map(|index| path(index, arm_count))
                .collect::<Vec<_>>();

            // No arm's path passes through another arm's position, so every
            // arm ends at its own leaf.
            for (a, first) in paths.iter().enumerate() {
                for (b, second) in paths.iter().enumerate() {
                    assert!(
                        a == b || !second.starts_with(first),
                        "{arm_count} arms: {paths:?}",
                    );
                }
            }

            // Every `EitherView` has both sides filled, so all arms build
            // the same type.
            let filled = paths
                .iter()
                .map(|path| 0.5_f64.powi(i32::try_from(path.len()).unwrap()))
                .sum::<f64>();
            assert!(
                (filled - 1.0).abs() < f64::EPSILON,
                "{arm_count} arms: {paths:?}",
            );

            let bound = arm_count.next_power_of_two().trailing_zeros() as usize;
            assert!(
                paths.iter().all(|path| path.len() <= bound),
                "{arm_count} arms: {paths:?}",
            );
        }
    }
}

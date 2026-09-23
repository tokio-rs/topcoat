use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, ViewFirst, ViewSwap};

pin_project! {
    /// Unifies the branch values of an `if`/`else` or `match` in node
    /// position: the branches build different types, but only the taken one
    /// is driven.
    ///
    /// `match` arms nest `Right`s to give every arm a distinct position in
    /// one type.
    #[project = EitherViewProj]
    pub enum EitherView<A, B> {
        /// The first branch.
        Left { #[pin] view: A },
        /// The second branch, or the nested remaining arms of a `match`.
        Right { #[pin] view: B },
    }
}

impl<A, B> EitherView<A, B> {
    /// Wraps the view of the first branch.
    #[must_use]
    pub fn left(view: A) -> Self {
        Self::Left { view }
    }

    /// Wraps the view of the second branch.
    #[must_use]
    pub fn right(view: B) -> Self {
        Self::Right { view }
    }
}

impl<A, B> View for EitherView<A, B>
where
    A: View,
    B: View,
{
    fn poll_first(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        match self.project() {
            EitherViewProj::Left { view } => view.poll_first(cx),
            EitherViewProj::Right { view } => view.poll_first(cx),
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        match self.project() {
            EitherViewProj::Left { view } => view.poll_swap(cx),
            EitherViewProj::Right { view } => view.poll_swap(cx),
        }
    }
}

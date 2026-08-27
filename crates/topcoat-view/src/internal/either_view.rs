use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{Step, View};

pin_project! {
    /// Unifies the branch values of an `if`/`else` or `match` in node
    /// position: the branches build different types, but only the taken one
    /// is driven.
    ///
    /// `match` arms nest `Right`s to give every arm a distinct position in
    /// one type.
    #[project = EitherViewProj]
    pub enum EitherView<A, B> {
        Left { #[pin] view: A },
        Right { #[pin] view: B },
    }
}

impl<A, B> EitherView<A, B> {
    #[must_use]
    pub fn left(view: A) -> Self {
        Self::Left { view }
    }

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
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Step>> {
        match self.project() {
            EitherViewProj::Left { view } => view.poll(cx),
            EitherViewProj::Right { view } => view.poll(cx),
        }
    }
}

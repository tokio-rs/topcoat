use std::{mem, pin::Pin};

use topcoat_core::context::Cx;

use crate::{BoxView, View, ViewBuffer};

/// The child content a component invocation passes to its component.
///
/// The children of an invocation reach the component as this lazy view in
/// its props. The component decides where they render by interpolating the
/// value into its own template, which drives the children concurrently with
/// the rest of the template; children that are never interpolated never
/// run.
pub struct Child<'a> {
    state: State<'a>,
}

/// A build deferred until the child is interpolated, run with the request
/// context and buffer of the template interpolating it.
type Build<'a> = Box<dyn FnOnce(&'a Cx, &'a ViewBuffer) -> BoxView<'a> + Send + 'a>;

/// Whether the child's view exists yet.
enum State<'a> {
    /// The view, built where the child was created.
    View(BoxView<'a>),
    /// The deferred build.
    Lazy(Build<'a>),
}

impl<'a> Child<'a> {
    /// Wraps a view as a component's child content.
    #[must_use]
    pub fn new(view: impl View + 'a) -> Self {
        Self {
            state: State::View(Box::pin(view)),
        }
    }

    /// Defers building the child content until it is interpolated.
    ///
    /// `build` runs with the request context and buffer of the template
    /// interpolating the child, so the view it builds sees the context of
    /// where it renders rather than where it was created. A caller provides
    /// context to the child by interpolating it under a derived context.
    #[must_use]
    pub fn lazy(build: impl FnOnce(&'a Cx, &'a ViewBuffer) -> BoxView<'a> + Send + 'a) -> Self {
        Self {
            state: State::Lazy(Box::new(build)),
        }
    }

    /// Returns the child's view, building it against `cx` and `buf` if it
    /// was deferred.
    pub(crate) fn view(&mut self, cx: &'a Cx, buf: &'a ViewBuffer) -> Pin<&mut (dyn View + 'a)> {
        if let State::Lazy(_) = self.state {
            let State::Lazy(build) = mem::replace(&mut self.state, State::View(Box::pin(())))
            else {
                unreachable!("checked to be deferred");
            };
            self.state = State::View(build(cx, buf));
        }
        match &mut self.state {
            State::View(view) => view.as_mut(),
            State::Lazy(_) => unreachable!("built above"),
        }
    }
}

/// No child content: renders nothing, like a component invoked without
/// children.
impl Default for Child<'_> {
    fn default() -> Self {
        Self::new(())
    }
}

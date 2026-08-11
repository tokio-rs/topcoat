//! Hand desugared prototype of the streaming SSR design in `DESIGN.md`.
//!
//! The crate validates the component future model: a component compiles to one
//! async function, the body followed by a render loop that suspends between
//! passes, with children stored as futures in the parent's frame. Everything
//! the `#[component]` and `view!` macros would generate is written by hand
//! here, and a manual driver runs passes deterministically so tests control
//! when every piece of I/O completes.
//!
//! The correspondence to the design:
//!
//! - A component is `async fn name(cx: Cx, me: CompId, ...props) -> Result`.
//!   The body runs once; the render loop renders once per pass and awaits
//!   [`pass_boundary`].
//! - [`Children`] is the frame local holding child futures, keyed the way the
//!   identity system keys invocations. [`Children::sweep`] is eviction.
//! - [`Defer`] is the deferred load handle; [`Deferred::Ready`] hands out `&T`.
//! - [`Driver`] is the request loop: snapshot deferred completions, advance the
//!   pass, poll the tree until it seals, assemble output from per component
//!   slots.
//!
//! Two deliberate simplifications, both noted where they apply: deferred
//! futures are `'static` instead of `'cx` (safe handoff of a `'cx` future to
//! the driver is part of the pass protocol open question), and the runtime is
//! single threaded (`Rc`, no `Send` validation).
//!
//! # Compile-time rejections
//!
//! The strategy's compile-time claims, checked as `compile_fail` tests.
//!
//! Render positions are closures that are not async, so a load in render code
//! does not compile:
//!
//! ```compile_fail,E0728
//! async fn load() -> String { String::new() }
//! async fn render() {
//!     let mut out = String::new();
//!     streaming_model::interp(&mut out, || load().await);
//! }
//! ```
//!
//! A child outlives the pass, so a prop borrowing a per pass temporary does not
//! compile:
//!
//! ```compile_fail,E0597
//! use streaming_model::*;
//! async fn child(cx: Cx, me: CompId, s: &str) -> Result {
//!     let mount = cx.register(me, "child");
//!     let mut children = Children::new();
//!     loop {
//!         cx.finish_render(&mount, s.to_string());
//!         pass_boundary(&cx, &mount, &mut children).await?;
//!     }
//! }
//! async fn parent(cx: Cx, me: CompId) -> Result {
//!     let mount = cx.register(me, "parent");
//!     let mut children = Children::new();
//!     loop {
//!         let mut out = String::new();
//!         let tmp = format!("pass {}", cx.pass());
//!         children.advance(&cx, &mut out, "c", |id| Box::pin(child(cx.clone(), id, &tmp)))?;
//!         cx.finish_render(&mount, out);
//!         pass_boundary(&cx, &mount, &mut children).await?;
//!     }
//! }
//! ```
//!
//! A deferred future runs while the component is suspended, so it cannot
//! borrow the component's locals:
//!
//! ```compile_fail,E0597
//! use streaming_model::*;
//! async fn comp(cx: Cx, me: CompId) -> Result {
//!     let mount = cx.register(me, "comp");
//!     let title = String::from("the menu");
//!     let title_ref = &title;
//!     let _d: Defer<usize> = Defer::spawn(&cx, "d", async move { title_ref.len() });
//!     let mut children = Children::new();
//!     loop {
//!         cx.finish_render(&mount, String::new());
//!         pass_boundary(&cx, &mount, &mut children).await?;
//!     }
//! }
//! ```

mod children;
mod cx;
mod defer;
mod driver;
mod error;
mod trigger;

pub use children::*;
pub use cx::*;
pub use defer::*;
pub use driver::*;
pub use error::*;
pub use trigger::*;

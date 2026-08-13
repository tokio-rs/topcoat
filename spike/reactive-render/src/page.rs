//! Hand-expanded "generated" code for one demo page: what the `view!`
//! and `#[component]` macros would emit under DESIGN.md, written by hand
//! so the compiler can judge the shapes. Each item documents the
//! Topcoat-level API it checks.
//!
//! The expansion shape here deviates from DESIGN.md's sketch in one
//! load-bearing way found by this spike (README finding 2): the frame
//! set cannot be declared at the top of the body, before the locals its
//! entries borrow, because dropck is not path-sensitive. The tail
//! `view!` expansion is instead an inner block entered after the body's
//! locals, and the set lives there.

use std::future::Future;

use crate::scope::{with, Part, Result};
use crate::set::{defer, run_node, Cx, Deferred, Fill, RefreshSet};

pub struct User {
    pub name: String,
}

/// A future that stands in for external work: pending for `delay`
/// polls, then ready. In the real driver an external completion wakes
/// the task; the spike's driver just keeps sweeping.
pub struct Slow<T: Unpin> {
    delay: u32,
    value: Option<T>,
}

impl<T: Unpin> Future for Slow<T> {
    type Output = T;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<T> {
        if self.delay == 0 {
            std::task::Poll::Ready(self.value.take().expect("polled after ready"))
        } else {
            self.delay -= 1;
            std::task::Poll::Pending
        }
    }
}

fn slow<T: Unpin>(delay: u32, value: T) -> Slow<T> {
    Slow {
        delay,
        value: Some(value),
    }
}

async fn current_user(_cx: &Cx) -> Result<User> {
    slow(
        1,
        User {
            name: "Ada".to_string(),
        },
    )
    .await;
    Ok(User {
        name: "Ada".to_string(),
    })
}

async fn orders(cx: &Cx, user: &User) -> Result<Vec<String>> {
    slow(8, ()).await;
    if cx.fail_orders {
        Err(format!("orders backend down for {}", user.name))
    } else {
        Ok(vec!["espresso machine".to_string(), "grinder".to_string()])
    }
}

async fn drinks(_cx: &Cx) -> Result<String> {
    slow(12, ()).await;
    Ok("Mojito, Flat White".to_string())
}

async fn feed_items(_cx: &Cx) -> Result<String> {
    slow(16, ()).await;
    Ok("3 new reviews".to_string())
}

/// Topcoat-level API under check:
///
/// ```text
/// #[component]
/// async fn avatar(cx: &Cx, user: &User, brand: &str) -> Result {
///     view! { <img alt=(&user.name) title=(brand)/> }
/// }
/// ```
///
/// Checks: the generated `render<'frame>` signature with props borrowing
/// the CALLER's locals (`'frame` is the invoking frame, not `'cx`), the
/// fill-then-continue shape, and the empty refresh phase as the
/// non-streaming fast path.
pub struct AvatarProps<'frame> {
    pub user: &'frame User,
    pub brand: &'frame str,
}

pub fn avatar<'frame>(
    _cx: &'frame Cx,
    props: AvatarProps<'frame>,
    fill: Fill,
) -> impl Future<Output = Result> + Send + 'frame {
    async move {
        let refresh = RefreshSet::new();
        let view = with(|s| {
            s.build(vec![
                Part::Html("<img alt=\""),
                Part::Text(props.user.name.clone()),
                Part::Html("\" title=\""),
                Part::Text(props.brand.to_string()),
                Part::Html("\"/>"),
            ])
        });
        fill.fill(view);
        // Finding 1 (README): bind the run future's result instead of
        // leaving `refresh.run().await` in tail position, because tail
        // temporaries outlive the block's locals.
        let done = refresh.run().await;
        done
    }
}

/// Topcoat-level API under check:
///
/// ```text
/// #[component]
/// async fn profile(cx: &Cx, brand: &str) -> Result {
///     let user = current_user(cx).await?;
///     view! {
///         <h2>(&user.name)</h2>
///         avatar(user: &user, brand: brand)          // fused invocation
///         live match defer(orders(cx, &user)) {      // reactive node
///             Deferred::Pending => <p>"orders loading..."</p>
///             Deferred::Ready(orders) => {
///                 for order in orders? { <li>(order)</li> }
///             }
///         }
///     }
/// }
/// ```
///
/// Checks: body locals in the outer scope with the frame set in the
/// tail expansion's inner block (README finding 2); a fused child whose
/// props borrow the body's `user`; a reactive node pushed, never awaited
/// in the hoist; the arms closure with its own arm frame, borrowing the
/// body's `user` through a per-call `Copy` reference; the arm's `?` as
/// the rethrow; and the barrier-then-burst-then-fill order.
pub struct ProfileProps<'frame> {
    pub brand: &'frame str,
}

pub fn profile<'frame>(
    cx: &'frame Cx,
    props: ProfileProps<'frame>,
    fill: Fill,
) -> impl Future<Output = Result> + Send + 'frame {
    async move {
        // The body, unchanged: its locals live in the outermost scope.
        let user = current_user(cx).await?;

        // The tail `view!` expansion: an inner block owning the set.
        {
            let mut refresh = RefreshSet::new();

            // Hoist: fused child and reactive node, nothing awaited.
            let expr0 = user.name.clone();
            let child0 = refresh.invoke(|f| {
                avatar(
                    cx,
                    AvatarProps {
                        user: &user,
                        brand: props.brand,
                    },
                    f,
                )
            });

            let (node0, node0_slot) = with(|s| s.reserve());
            let node0_ticket = refresh.ticket();
            let reactive0 = defer(cx, orders(cx, &user));
            let user_ref = &user;
            refresh.push(run_node(reactive0, move |state| {
                // Per-call copies moved into the arm future (README
                // finding 3): `Copy` slot, ticket, and reference.
                let slot = node0_slot;
                let ticket = node0_ticket;
                let user = user_ref;
                async move {
                    // One run of the arms is a frame of its own.
                    let mut arm = RefreshSet::new();
                    let view = match state {
                        Deferred::Pending => {
                            with(|s| s.build(vec![Part::Html("<p>orders loading...</p>")]))
                        }
                        Deferred::Ready(orders) => {
                            // The arm's `?` is real: it fails the node,
                            // and the failure climbs the frame tree.
                            let orders = orders?;
                            with(|s| {
                                let mut parts = vec![Part::Html("<ul>")];
                                for order in &orders {
                                    parts.push(Part::Html("<li>"));
                                    parts.push(Part::Text(format!("{order} for {}", user.name)));
                                    parts.push(Part::Html("</li>"));
                                }
                                parts.push(Part::Html("</ul>"));
                                s.build(parts)
                            })
                        }
                    };
                    arm.barrier().await?;
                    slot.refill(view);
                    ticket.hand_over();
                    let done = arm.run().await;
                    done
                }
            }));

            // Join: wait for every ticket, then burst, then the yield.
            refresh.barrier().await?;
            let view = with(|s| {
                s.build(vec![
                    Part::Html("<h2>"),
                    Part::Text(expr0),
                    Part::Html("</h2>"),
                    Part::Splice(child0),
                    Part::Splice(node0),
                ])
            });
            fill.fill(view);

            // The refresh phase: `user` is alive across this await; that
            // is the point of not returning.
            let done = refresh.run().await;
            done
        }
    }
}

/// Topcoat-level API under check:
///
/// ```text
/// #[component]
/// async fn activity_feed(cx: &Cx) -> Result {
///     view! {
///         <aside>"The feed "
///             live match defer(feed_items(cx)) {
///                 Deferred::Pending => <em>"feed loading..."</em>
///                 Deferred::Ready(items) => <em>(items?)</em>
///             }
///         </aside>
///     }
/// }
/// ```
///
/// Checks: a component with its own deferred content, used UNFUSED (see
/// `page`): its swaps must keep landing wherever its handle is spliced,
/// including inside a `live` arm that was swapped in later.
pub fn activity_feed<'frame>(
    cx: &'frame Cx,
    fill: Fill,
) -> impl Future<Output = Result> + Send + 'frame {
    async move {
        let mut refresh = RefreshSet::new();

        let (node0, node0_slot) = with(|s| s.reserve());
        let node0_ticket = refresh.ticket();
        let reactive0 = defer(cx, feed_items(cx));
        refresh.push(run_node(reactive0, move |state| {
            let slot = node0_slot;
            let ticket = node0_ticket;
            async move {
                let mut arm = RefreshSet::new();
                let view = match state {
                    Deferred::Pending => {
                        with(|s| s.build(vec![Part::Html("<em>feed loading...</em>")]))
                    }
                    Deferred::Ready(items) => {
                        let items = items?;
                        with(|s| {
                            s.build(vec![
                                Part::Html("<em>"),
                                Part::Text(items),
                                Part::Html("</em>"),
                            ])
                        })
                    }
                };
                arm.barrier().await?;
                slot.refill(view);
                ticket.hand_over();
                let done = arm.run().await;
                done
            }
        }));

        refresh.barrier().await?;
        let view = with(|s| {
            s.build(vec![
                Part::Html("<aside>The feed "),
                Part::Splice(node0),
                Part::Html("</aside>"),
            ])
        });
        fill.fill(view);
        let done = refresh.run().await;
        done
    }
}

/// Topcoat-level API under check:
///
/// ```text
/// #[page]
/// async fn page(cx: &Cx) -> Result {
///     let brand = brand_name();
///     let feed = view! { activity_feed() };          // unfused handle
///     view! {
///         <main>
///             profile(brand: &brand)
///             live match defer(drinks(cx)) {
///                 Deferred::Pending => {
///                     <div class="dim">(feed.clone()) " drinks loading..."</div>
///                 }
///                 Deferred::Ready(drinks) => {
///                     <div>(feed.clone()) " " (drinks?)</div>
///                 }
///             }
///         </main>
///     }
/// }
/// ```
///
/// Checks: the unfused handle (`adopt` at the binding, no `?`), handle
/// clones spliced into BOTH arms of a `live match` (the shared-`feed`
/// pattern: one render, splice-after-delivery served from the cache,
/// the swap neither cancelling nor restarting it), and the arm frame
/// owning the splices while the body frame owns the tender. The arms
/// closure captures the handle BY CLONE, not by reference (README
/// finding 4).
pub fn page<'frame>(cx: &'frame Cx, fill: Fill) -> impl Future<Output = Result> + Send + 'frame {
    async move {
        // Body locals, outer scope.
        let brand = String::from("Topcoat Cafe");

        {
            let mut refresh = RefreshSet::new();

            // `let feed = view! { activity_feed() };` in the guide. The
            // handle binding lives with the set here; the general
            // mid-body desugar is README finding 2.
            let feed = refresh.adopt(|f| activity_feed(cx, f));

            // Hoist.
            let child0 = refresh.invoke(|f| profile(cx, ProfileProps { brand: &brand }, f));

            let (node0, node0_slot) = with(|s| s.reserve());
            let node0_ticket = refresh.ticket();
            let reactive0 = defer(cx, drinks(cx));
            let feed_for_arms = feed.clone();
            refresh.push(run_node(reactive0, move |state| {
                let slot = node0_slot;
                let ticket = node0_ticket;
                let feed = feed_for_arms.clone();
                async move {
                    let mut arm = RefreshSet::new();
                    let view = match state {
                        Deferred::Pending => {
                            let feed_view = arm.splice(feed);
                            with(|s| {
                                s.build(vec![
                                    Part::Html("<div class=\"dim\">"),
                                    Part::Splice(feed_view),
                                    Part::Html(" drinks loading...</div>"),
                                ])
                            })
                        }
                        Deferred::Ready(drinks) => {
                            let drinks = drinks?;
                            let feed_view = arm.splice(feed);
                            with(|s| {
                                s.build(vec![
                                    Part::Html("<div>"),
                                    Part::Splice(feed_view),
                                    Part::Html(" "),
                                    Part::Text(drinks),
                                    Part::Html("</div>"),
                                ])
                            })
                        }
                    };
                    arm.barrier().await?;
                    slot.refill(view);
                    ticket.hand_over();
                    let done = arm.run().await;
                    done
                }
            }));

            refresh.barrier().await?;
            let view = with(|s| {
                s.build(vec![
                    Part::Html("<main>"),
                    Part::Splice(child0),
                    Part::Splice(node0),
                    Part::Html("</main>"),
                ])
            });
            fill.fill(view);
            let done = refresh.run().await;
            done
        }
    }
}

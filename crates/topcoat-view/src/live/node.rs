use std::{
    future::{Future, poll_fn},
    pin::{Pin, pin},
    task::Poll,
};

use topcoat_core::error::Result;

use crate::live::Reactive;

/// What one round of a reactive node's race resolved to.
pub enum Raced<S> {
    /// The reactive expression yielded, a replacement state or retirement.
    State(Option<S>),
    /// The shown arm finished its remaining live work first.
    ArmDone(Result<()>),
}

/// Races a shown arm's remaining live work against the reactive
/// expression's next state, the next state winning ties.
///
/// One round of a reactive node's loop; the caller decides what a new
/// state, retirement, or a finished arm means. Both futures stay usable
/// after the race, so the loser can be awaited or dropped.
pub async fn race_node<S, A, N>(mut arm: Pin<&mut A>, mut next: Pin<&mut N>) -> Raced<S>
where
    A: Future<Output = Result<()>>,
    N: Future<Output = Option<S>>,
{
    poll_fn(|task| {
        if let Poll::Ready(state) = next.as_mut().poll(task) {
            return Poll::Ready(Raced::State(state));
        }
        if let Poll::Ready(result) = arm.as_mut().poll(task) {
            return Poll::Ready(Raced::ArmDone(result));
        }
        Poll::Pending
    })
    .await
}

/// Drives one reactive construct: await a state, run the arms, then race the
/// shown arm's remaining work against the next state.
///
/// A new state drops the arm's future, and with it the arm's frame,
/// cancelling the replaced subtree's outstanding work; retirement (`None`)
/// lets the shown arm finish its remaining live work and completes the node.
///
/// `arms` is a plain `FnMut` returning a `Send` future, so everything an arm
/// captures must be a per-call copy: `Copy` slots, tickets, and references,
/// and handle clones.
///
/// # Errors
///
/// Returns the first error an arm run reports, which is how a failure inside
/// the construct climbs to the enclosing frame.
pub async fn run_node<R, A, F>(mut reactive: R, mut arms: A) -> Result<()>
where
    R: Reactive,
    A: FnMut(R::State) -> F + Send,
    F: Future<Output = Result<()>> + Send,
{
    let Some(mut state) = reactive.next_state().await else {
        return Ok(());
    };
    loop {
        let mut arm = pin!(arms(state));
        let mut next = pin!(reactive.next_state());
        match race_node(arm.as_mut(), next.as_mut()).await {
            // A new state: dropping `arm` is the cancellation of the
            // replaced arm's frame and everything it started.
            Raced::State(Some(new_state)) => {
                state = new_state;
            }
            // Retired: finish the shown arm's remaining live work.
            Raced::State(None) => return arm.await,
            // The arm finished first: keep listening for the next state.
            Raced::ArmDone(result) => {
                result?;
                match next.await {
                    Some(new_state) => state = new_state,
                    None => return Ok(()),
                }
            }
        }
    }
}

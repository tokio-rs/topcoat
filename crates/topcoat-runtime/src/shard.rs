use serde::Deserialize;

use crate::{Arguments, SignalValues};

/// The body of a request re-rendering a shard: the current values of its
/// arguments and of the signals its content created.
///
/// The identity of the shard invocation travels separately, in the
/// request's identity header.
#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "Arguments<A>: Deserialize<'de>"))]
pub struct ShardRequest<A> {
    args: Arguments<A>,
    #[serde(default)]
    signals: SignalValues,
}

impl<A> ShardRequest<A> {
    /// Splits the request into the arguments to hand the shard body and the
    /// signal values to register on its request context.
    pub fn into_parts(self) -> (A, SignalValues) {
        (self.args.0, self.signals)
    }
}

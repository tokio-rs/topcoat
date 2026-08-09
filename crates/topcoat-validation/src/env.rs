/// The environment a validator or validation run targets.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationEnv {
    /// Runs only on the server.
    Server,
    /// Runs only in the browser or other client context.
    Client,
    /// Runs on both the server and the client.
    #[default]
    Both,
}

impl ValidationEnv {
    /// Returns `true` if a validator configured for `self` should run when the
    /// schema is validated in `env`.
    ///
    /// A validator set to [`Both`](Self::Both) always runs, and a validation
    /// run configured as [`Both`](Self::Both) runs every validator.
    #[must_use]
    pub fn includes(self, env: Self) -> bool {
        match (self, env) {
            (Self::Both, _) | (_, Self::Both) => true,
            (a, b) => a == b,
        }
    }
}

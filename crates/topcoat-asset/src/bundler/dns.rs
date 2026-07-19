use std::{collections::HashMap, net::SocketAddr, sync::Mutex};

use http::Uri;
use ureq::{
    config::Config,
    unversioned::{
        resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver},
        transport::NextTimeout,
    },
};

/// A [`Resolver`] that memoizes DNS lookups per host for the life of the agent.
///
/// ureq resolves a request's host before consulting its connection pool, so
/// every request pays a fresh lookup even when a pooled keep-alive connection
/// is about to be reused. A bundle run typically fetches many assets from a
/// handful of hosts, and on systems with slow or partially unreachable DNS
/// each lookup can take seconds. Caching per host bounds that cost to one
/// lookup per host per run.
///
/// Failed lookups are not cached, so a transient DNS error on one asset does
/// not poison the rest of the run.
// The `ureq::unversioned` module is exempt from ureq's semver guarantees; a
// breaking change there surfaces as a compile error in this file.
#[derive(Debug, Default)]
pub struct CachingResolver {
    inner: DefaultResolver,
    cache: Mutex<HashMap<String, Vec<SocketAddr>>>,
}

impl CachingResolver {
    fn lock_cache(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<SocketAddr>>> {
        self.cache.lock().expect("resolver cache lock poisoned")
    }
}

impl Resolver for CachingResolver {
    fn resolve(
        &self,
        uri: &Uri,
        config: &Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let key = match (uri.scheme(), uri.authority()) {
            (Some(scheme), Some(authority)) => DefaultResolver::host_and_port(scheme, authority),
            _ => None,
        };
        // Without a resolvable authority, let the default resolver produce
        // its regular error.
        let Some(key) = key else {
            return self.inner.resolve(uri, config, timeout);
        };

        if let Some(addrs) = self.lock_cache().get(&key) {
            let mut resolved = self.empty();
            for addr in addrs {
                resolved.push(*addr);
            }
            return Ok(resolved);
        }

        let resolved = self.inner.resolve(uri, config, timeout)?;
        self.lock_cache()
            .insert(key, resolved.iter().copied().collect());
        Ok(resolved)
    }
}

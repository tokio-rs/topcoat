use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use http::{HeaderMap, HeaderName, header};
use topcoat_core::context::{Cx, try_request_context};

use crate::{remote_addr, request::headers};

/// The reverse proxies the router trusts to report the client's address.
///
/// A request that arrives through a reverse proxy (a load balancer, a CDN,
/// nginx in front of the application) connects from the proxy's address, and
/// the proxy passes the client's address along in a header. Any client can
/// send that header too, so the router only reads it on a connection it knows
/// comes from a proxy. This value names those proxies. Register it with
/// [`RouterBuilder::trusted_proxies`](crate::RouterBuilder::trusted_proxies),
/// and [`client_ip`](crate::request::client_ip) resolves the client's address
/// through them.
///
/// A proxy is trusted by the [network](Self::networks) it connects from, or,
/// when its address is not fixed, by its position: the
/// [nearest](Self::nearest) proxies to the application, counted from the
/// connection's peer outward. Prefer networks, since positional trust also
/// covers a client that manages to take a proxy's position. The default
/// trusts no proxy, so the client address is the connection's peer.
///
/// Trusting a proxy means trusting it to append the address it sees to the
/// forwarding [header](Self::header) on every request it passes on. A proxy
/// that forwards the header a client sent, unchanged, lets that client choose
/// its own address.
///
/// # Examples
///
/// nginx on a private network in front of the application:
///
/// ```rust
/// use topcoat::router::{Router, TrustedProxies};
///
/// let router = Router::builder()
///     .trusted_proxies(TrustedProxies::new().networks(["10.0.0.0/8"]))
///     .build();
/// ```
///
/// A managed load balancer whose addresses are not known ahead of time, as
/// the only proxy between the client and the application:
///
/// ```rust
/// use topcoat::router::{Router, TrustedProxies};
///
/// let router = Router::builder()
///     .trusted_proxies(TrustedProxies::new().nearest(1))
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct TrustedProxies {
    networks: Vec<Network>,
    nearest: usize,
    header: ForwardedHeader,
}

impl TrustedProxies {
    /// Creates the default policy: no proxy is trusted, and the client
    /// address is the connection's peer.
    ///
    /// Add proxies with [`networks`](Self::networks) and
    /// [`nearest`](Self::nearest).
    #[must_use]
    pub fn new() -> Self {
        Self {
            networks: Vec::new(),
            nearest: 0,
            header: ForwardedHeader::XForwardedFor,
        }
    }

    /// Trusts every proxy connecting from one of `networks`.
    ///
    /// Each value is an address with a prefix length in CIDR notation, like
    /// `"10.0.0.0/8"` or `"fd00::/8"`, or a single address like
    /// `"127.0.0.1"`. IPv4 and IPv6 networks are separate: an IPv6 network
    /// never contains an IPv4 address. The IPv4-mapped IPv6 form
    /// (`::ffff:10.0.0.1`), which a dual-stack socket reports IPv4 peers in,
    /// counts as IPv4 both here and in the addresses compared against it.
    ///
    /// # Panics
    ///
    /// Panics if a value is not an address or a network in CIDR notation.
    #[must_use]
    #[track_caller]
    pub fn networks<I>(mut self, networks: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        for network in networks {
            let network = network.as_ref();
            match Network::parse(network) {
                Some(network) => self.networks.push(network),
                None => panic!("invalid trusted proxy network `{network}`"),
            }
        }
        self
    }

    /// Trusts the `count` proxies nearest to the application, whatever
    /// their addresses.
    ///
    /// The connection's peer is the first; each address the forwarding header
    /// lists, read from its end, is the next one out. Use this for a proxy
    /// whose address is not fixed, like a managed load balancer, or one that
    /// connects over a Unix socket and so has no address at all.
    ///
    /// Trust by position is only safe when every request passes through
    /// exactly `count` proxies, since whatever sits at those positions is
    /// trusted, the client included. With a CDN in front of a load balancer,
    /// `count` is two; if a client can also reach the load balancer directly,
    /// bypassing the CDN, it takes the CDN's position and the address it
    /// sends in the header is believed. Prefer [`networks`](Self::networks)
    /// wherever the proxies' addresses are known, and reserve this for a
    /// fixed topology the application cannot be reached around.
    #[must_use]
    pub fn nearest(mut self, count: usize) -> Self {
        self.nearest = count;
        self
    }

    /// Sets the header the client address is read from.
    ///
    /// The default is [`ForwardedHeader::XForwardedFor`]. Only this header is
    /// read: a proxy manages one of them, and reading another would trust a
    /// value the client may have sent.
    #[must_use]
    pub fn header(mut self, header: ForwardedHeader) -> Self {
        self.header = header;
        self
    }

    /// Resolves the client address of the request on `cx`.
    pub(crate) fn resolve(&self, cx: &Cx) -> Option<IpAddr> {
        let remote = remote_addr(cx).map(|addr| addr.ip());
        self.client_ip(remote, headers(cx))
    }

    /// Resolves the client address of a request that arrived from `remote`
    /// with `headers`.
    ///
    /// Walks from the peer outward through the addresses the forwarding
    /// header lists, skipping every trusted proxy; the first untrusted
    /// address is the client's. When every address is trusted, the farthest
    /// one is returned. An entry that carries no readable address ends the
    /// walk with no client address at all.
    pub(crate) fn client_ip(&self, remote: Option<IpAddr>, headers: &HeaderMap) -> Option<IpAddr> {
        let remote = remote.map(|ip| ip.to_canonical());
        if !self.is_trusted(remote, 0) {
            return remote;
        }

        // A value that is not visible ASCII carries no address, so it is read
        // as one unparsable entry.
        let values = headers
            .get_all(self.header.name())
            .iter()
            .map(|value| value.to_str().unwrap_or("?"));

        match self.header {
            ForwardedHeader::Single(_) => {
                // Exactly one field holding exactly one address; any other
                // shape is ambiguous, so it names none.
                let mut values = values;
                let value = values.next()?;
                if values.next().is_some() {
                    return None;
                }
                parse_node(value.trim())
            }
            ForwardedHeader::Forwarded => {
                // An element's quoted strings may contain the list and
                // parameter delimiters, so the split reads front to back with
                // the quoting in mind, and the walk goes over the addresses
                // from the back.
                let addresses: Vec<Option<IpAddr>> = values
                    .flat_map(|value| {
                        split_quoted(value, ',')
                            .filter(|element| !element.trim().is_empty())
                            .map(|element| {
                                forwarded_for(element).and_then(|node| parse_node(&node))
                            })
                    })
                    .collect();
                self.walk(remote, addresses.into_iter().rev())
            }
            ForwardedHeader::XForwardedFor => {
                // The entries from the one nearest to the application (the
                // end of the last value) outward.
                let entries = values.rev().flat_map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|entry| !entry.is_empty())
                        .rev()
                });
                self.walk(remote, entries.map(parse_node))
            }
        }
    }

    /// Walks outward from the peer at `remote` over `addresses`, the
    /// forwarding header's entries from the one nearest to the application,
    /// to the first untrusted address.
    fn walk(
        &self,
        remote: Option<IpAddr>,
        addresses: impl Iterator<Item = Option<IpAddr>>,
    ) -> Option<IpAddr> {
        let mut farthest = remote;
        for (hop, address) in addresses.enumerate() {
            let ip = address?;
            if !self.is_trusted(Some(ip), hop + 1) {
                return Some(ip);
            }
            farthest = Some(ip);
        }
        farthest
    }

    /// Whether the proxy at `addr`, `hop` positions out from the
    /// application, is trusted.
    fn is_trusted(&self, addr: Option<IpAddr>, hop: usize) -> bool {
        hop < self.nearest
            || addr.is_some_and(|ip| self.networks.iter().any(|network| network.contains(ip)))
    }
}

/// Trusts no proxy.
impl Default for TrustedProxies {
    fn default() -> Self {
        Self::new()
    }
}

/// The client's address as resolved when the request arrived, stored on the
/// request context of every dispatch.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientIp(pub(crate) Option<IpAddr>);

/// Returns the IP address of the client behind the current request, or `None`
/// when it cannot be determined.
///
/// Without trusted proxies this is the address of the connection's peer, as
/// [`remote_addr`] returns it. With [`TrustedProxies`] registered on the
/// router, a request arriving from a trusted proxy resolves to the address
/// the proxy reports in the forwarding header, walking past every trusted
/// proxy in the chain to the first address that is not one.
///
/// The router resolves the address once, as the request arrives and before
/// any layer runs, so every reader over the life of the request sees the same
/// client, whatever a layer does to the headers later. A context no router
/// dispatched has no client.
///
/// The address is unknown when the peer is unknown and not trusted by
/// position, or when a trusted proxy reported an entry that carries no
/// readable address. An IPv4 address a dual-stack listener reports in its
/// IPv6-mapped form is returned as IPv4.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{client_ip, error::forbidden},
/// };
///
/// # fn is_banned(_ip: std::net::IpAddr) -> bool { false }
/// fn reject_banned(cx: &Cx) -> Result<()> {
///     match client_ip(cx) {
///         Some(ip) if is_banned(ip) => Err(forbidden().into()),
///         _ => Ok(()),
///     }
/// }
/// ```
#[must_use]
#[track_caller]
pub fn client_ip(cx: &Cx) -> Option<IpAddr> {
    try_request_context::<ClientIp>(cx)?.0
}

/// The header a trusted proxy reports the client address in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardedHeader {
    /// `X-Forwarded-For`, listing the client's address followed by the
    /// address of each proxy the request passed through after it, comma
    /// separated. Set by most proxies and load balancers; the default.
    XForwardedFor,
    /// `Forwarded` (RFC 7239), listing the same chain as `for=` parameters,
    /// one element per proxy.
    Forwarded,
    /// A header carrying the client's address alone, like `CF-Connecting-IP`
    /// or `True-Client-IP`.
    ///
    /// Once the connection's peer is trusted, the header's value is the
    /// client's address; there is no chain to walk. The header must appear
    /// exactly once, holding exactly one address, or it names none.
    ///
    /// The peer must set or overwrite the header itself, or verify that the
    /// proxy before it did. Trusting an internal load balancer that passes
    /// the header through unchanged vouches for nothing about a value that
    /// was supposedly set further out, since the client may have sent it.
    Single(HeaderName),
}

impl ForwardedHeader {
    /// The name of the header read.
    fn name(&self) -> &HeaderName {
        static X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");

        match self {
            Self::XForwardedFor => &X_FORWARDED_FOR,
            Self::Forwarded => &header::FORWARDED,
            Self::Single(name) => name,
        }
    }
}

/// A network in CIDR notation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Network {
    addr: IpAddr,
    prefix: u8,
}

impl Network {
    /// Parses `"addr/prefix"`, or a bare address as a network of one.
    ///
    /// A network of IPv4-mapped IPv6 addresses (`::ffff:10.0.0.0/104`) is
    /// read as the IPv4 network it maps to, the form addresses are compared
    /// in.
    fn parse(value: &str) -> Option<Self> {
        let (addr, prefix) = match value.split_once('/') {
            Some((addr, prefix)) => (addr.parse::<IpAddr>().ok()?, prefix.parse::<u8>().ok()?),
            None => {
                let addr = value.parse::<IpAddr>().ok()?;
                (addr, Self::bits(addr))
            }
        };
        if prefix > Self::bits(addr) {
            return None;
        }
        /// The prefix length of the `::ffff:0:0/96` block holding the mapped
        /// addresses; a longer prefix denotes a network within it.
        const MAPPED_PREFIX: u8 = 96;
        match (addr, addr.to_canonical()) {
            (IpAddr::V6(_), canonical @ IpAddr::V4(_)) if prefix >= MAPPED_PREFIX => Some(Self {
                addr: canonical,
                prefix: prefix - MAPPED_PREFIX,
            }),
            _ => Some(Self { addr, prefix }),
        }
    }

    /// Whether `ip`, in its canonical form, lies within the network.
    fn contains(self, ip: IpAddr) -> bool {
        match (self.addr, ip) {
            (IpAddr::V4(network), IpAddr::V4(ip)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(network) & mask == u32::from(ip) & mask
            }
            (IpAddr::V6(network), IpAddr::V6(ip)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(network) & mask == u128::from(ip) & mask
            }
            _ => false,
        }
    }

    /// The address length of `addr`'s family, the largest prefix it allows.
    fn bits(addr: IpAddr) -> u8 {
        match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        }
    }
}

/// Parses a node identifier as forwarding headers spell one: an address, or
/// an address with a port, where an IPv6 address with a port is in brackets.
/// A value like `unknown` or an obfuscated identifier carries no address,
/// and neither does any other form.
fn parse_node(value: &str) -> Option<IpAddr> {
    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip.to_canonical());
    }
    let (ip, port) = match value.strip_prefix('[') {
        Some(rest) => {
            let (ip, rest) = rest.split_once(']')?;
            let ip = IpAddr::V6(ip.parse::<Ipv6Addr>().ok()?);
            if rest.is_empty() {
                return Some(ip.to_canonical());
            }
            (ip, rest.strip_prefix(':')?)
        }
        None => {
            let (ip, port) = value.rsplit_once(':')?;
            (IpAddr::V4(ip.parse::<Ipv4Addr>().ok()?), port)
        }
    };
    is_node_port(port).then(|| ip.to_canonical())
}

/// Whether `value` is a port as a node identifier carries one: up to five
/// digits, or an obfuscated port starting with `_`.
fn is_node_port(value: &str) -> bool {
    match value.strip_prefix('_') {
        Some(obfuscated) => {
            !obfuscated.is_empty()
                && obfuscated
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        }
        None => (1..=5).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit()),
    }
}

/// Reads the `for` parameter out of one element of a `Forwarded` header,
/// unquoted, or `None` when the element has none or quotes it malformed.
fn forwarded_for(element: &str) -> Option<Cow<'_, str>> {
    split_quoted(element, ';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case("for")
            .then(|| unquote(value.trim()))?
    })
}

/// Splits `value` at each `delimiter` outside of a quoted string, since a
/// quoted string may contain the delimiter (escaped quotes included).
fn split_quoted(value: &str, delimiter: char) -> impl Iterator<Item = &str> {
    let mut quoted = false;
    let mut escaped = false;
    value.split(move |c: char| {
        if escaped {
            escaped = false;
        } else if quoted {
            match c {
                '\\' => escaped = true,
                '"' => quoted = false,
                _ => {}
            }
        } else if c == '"' {
            quoted = true;
        } else {
            return c == delimiter;
        }
        false
    })
}

/// Strips the quotes and escapes off a quoted string, or returns a token as
/// it is. A value with an opening quote but no closing one is malformed.
fn unquote(value: &str) -> Option<Cow<'_, str>> {
    let Some(rest) = value.strip_prefix('"') else {
        return Some(Cow::Borrowed(value));
    };
    let inner = rest.strip_suffix('"')?;
    if !inner.contains('\\') {
        return Some(Cow::Borrowed(inner));
    }
    let mut unescaped = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => unescaped.push(chars.next()?),
            c => unescaped.push(c),
        }
    }
    Some(Cow::Owned(unescaped))
}

#[cfg(test)]
mod tests {
    use http::HeaderValue;

    use super::*;

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    fn headers(values: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in values {
            headers.append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    fn resolve(proxies: &TrustedProxies, remote: Option<&str>, xff: &str) -> Option<IpAddr> {
        proxies.client_ip(remote.map(ip), &headers(&[("x-forwarded-for", xff)]))
    }

    // -- no trusted proxies --

    #[test]
    fn without_trusted_proxies_the_peer_is_the_client() {
        let proxies = TrustedProxies::new();
        assert_eq!(
            resolve(&proxies, Some("203.0.113.9"), "198.51.100.1"),
            Some(ip("203.0.113.9"))
        );
    }

    #[test]
    fn without_a_peer_or_trusted_proxies_the_client_is_unknown() {
        let proxies = TrustedProxies::new();
        assert_eq!(resolve(&proxies, None, "198.51.100.1"), None);
    }

    // -- trusted networks --

    #[test]
    fn a_trusted_peer_reports_the_client() {
        let proxies = TrustedProxies::new().networks(["10.0.0.0/8"]);
        assert_eq!(
            resolve(&proxies, Some("10.1.2.3"), "198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn an_untrusted_peer_is_the_client_whatever_the_header_says() {
        let proxies = TrustedProxies::new().networks(["10.0.0.0/8"]);
        assert_eq!(
            resolve(&proxies, Some("203.0.113.9"), "198.51.100.1"),
            Some(ip("203.0.113.9"))
        );
    }

    #[test]
    fn the_walk_skips_every_trusted_hop() {
        // Client -> CDN (trusted) -> load balancer (trusted, the peer).
        let proxies = TrustedProxies::new().networks(["10.0.0.0/8", "192.0.2.0/24"]);
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "198.51.100.1, 192.0.2.7"),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn the_walk_stops_at_the_first_untrusted_address() {
        // The client sent a header of its own, which the proxy appended to.
        let proxies = TrustedProxies::new().networks(["10.0.0.0/8"]);
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "1.1.1.1, 198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn a_chain_of_trusted_addresses_resolves_to_the_farthest() {
        let proxies = TrustedProxies::new().networks(["10.0.0.0/8"]);
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "10.0.0.5, 10.0.0.3"),
            Some(ip("10.0.0.5"))
        );
    }

    #[test]
    fn a_trusted_peer_without_a_header_is_the_client() {
        let proxies = TrustedProxies::new().networks(["10.0.0.0/8"]);
        assert_eq!(
            proxies.client_ip(Some(ip("10.0.0.1")), &HeaderMap::new()),
            Some(ip("10.0.0.1"))
        );
    }

    #[test]
    fn a_single_address_network_matches_that_address_only() {
        let proxies = TrustedProxies::new().networks(["10.0.0.1"]);
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
        assert_eq!(
            resolve(&proxies, Some("10.0.0.2"), "198.51.100.1"),
            Some(ip("10.0.0.2"))
        );
    }

    #[test]
    fn ipv6_networks_match() {
        let proxies = TrustedProxies::new().networks(["fd00::/8"]);
        assert_eq!(
            resolve(&proxies, Some("fd12::1"), "2001:db8::1"),
            Some(ip("2001:db8::1"))
        );
        assert_eq!(
            resolve(&proxies, Some("2001:db8::2"), "2001:db8::1"),
            Some(ip("2001:db8::2"))
        );
    }

    #[test]
    fn a_mapped_ipv4_peer_matches_an_ipv4_network() {
        // A dual-stack listener reports IPv4 peers in their mapped form.
        let proxies = TrustedProxies::new().networks(["10.0.0.0/8"]);
        assert_eq!(
            resolve(&proxies, Some("::ffff:10.0.0.1"), "::ffff:198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn a_mapped_ipv4_network_is_read_as_ipv4() {
        for network in ["::ffff:10.0.0.0/104", "::ffff:10.0.0.1"] {
            let proxies = TrustedProxies::new().networks([network]);
            for peer in ["10.0.0.1", "::ffff:10.0.0.1"] {
                assert_eq!(
                    resolve(&proxies, Some(peer), "198.51.100.1"),
                    Some(ip("198.51.100.1")),
                    "{network} should contain {peer}"
                );
            }
            assert_eq!(
                resolve(&proxies, Some("192.0.2.1"), "198.51.100.1"),
                Some(ip("192.0.2.1")),
                "{network} should not contain 192.0.2.1"
            );
        }
    }

    #[test]
    fn an_ipv6_network_never_contains_an_ipv4_address() {
        // The whole of IPv6 covers the mapped block, but the families stay
        // apart.
        let proxies = TrustedProxies::new().networks(["::/0"]);
        for peer in ["10.0.0.1", "::ffff:10.0.0.1"] {
            assert_eq!(
                resolve(&proxies, Some(peer), "198.51.100.1"),
                Some(ip("10.0.0.1"))
            );
        }
    }

    #[test]
    fn a_zero_prefix_matches_everything() {
        let proxies = TrustedProxies::new().networks(["0.0.0.0/0", "::/0"]);
        assert_eq!(
            resolve(&proxies, Some("203.0.113.9"), "198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
        assert_eq!(
            resolve(&proxies, Some("2001:db8::2"), "2001:db8::1"),
            Some(ip("2001:db8::1"))
        );
    }

    #[test]
    #[should_panic(expected = "invalid trusted proxy network `10.0.0.0/33`")]
    fn a_prefix_beyond_the_address_length_panics() {
        let _ = TrustedProxies::new().networks(["10.0.0.0/33"]);
    }

    #[test]
    #[should_panic(expected = "invalid trusted proxy network `proxy.internal`")]
    fn a_host_name_panics() {
        let _ = TrustedProxies::new().networks(["proxy.internal"]);
    }

    // -- nearest proxies --

    #[test]
    fn the_nearest_proxy_is_trusted_whatever_its_address() {
        let proxies = TrustedProxies::new().nearest(1);
        assert_eq!(
            resolve(&proxies, Some("203.0.113.9"), "198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn a_peer_without_an_address_counts_as_the_nearest_proxy() {
        // A proxy connecting over a Unix socket.
        let proxies = TrustedProxies::new().nearest(1);
        assert_eq!(
            resolve(&proxies, None, "198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn nearest_counts_hops_out_from_the_peer() {
        let proxies = TrustedProxies::new().nearest(2);
        assert_eq!(
            resolve(
                &proxies,
                Some("203.0.113.9"),
                "1.1.1.1, 198.51.100.1, 192.0.2.7"
            ),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn nearest_and_networks_combine() {
        // The load balancer has no fixed address; the CDN's network is known.
        let proxies = TrustedProxies::new().nearest(1).networks(["192.0.2.0/24"]);
        assert_eq!(
            resolve(&proxies, Some("203.0.113.9"), "198.51.100.1, 192.0.2.7"),
            Some(ip("198.51.100.1"))
        );
    }

    // -- header syntax --

    #[test]
    fn entries_span_several_header_lines() {
        let proxies = TrustedProxies::new().networks(["10.0.0.0/8"]);
        let headers = headers(&[
            ("x-forwarded-for", "198.51.100.1"),
            ("x-forwarded-for", "10.0.0.5, 10.0.0.3"),
        ]);
        assert_eq!(
            proxies.client_ip(Some(ip("10.0.0.1")), &headers),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn ports_and_brackets_are_stripped() {
        let proxies = TrustedProxies::new().nearest(1);
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "198.51.100.1:4242"),
            Some(ip("198.51.100.1"))
        );
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "[2001:db8::1]:4242"),
            Some(ip("2001:db8::1"))
        );
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "[2001:db8::1]"),
            Some(ip("2001:db8::1"))
        );
        // An obfuscated port still names the address.
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "[2001:db8::1]:_port1"),
            Some(ip("2001:db8::1"))
        );
    }

    #[test]
    fn a_malformed_node_leaves_the_client_unknown() {
        let proxies = TrustedProxies::new().nearest(1);
        for value in [
            "\"198.51.100.1\"",
            "198.51.100.1:",
            "198.51.100.1:123456",
            "198.51.100.1:http",
            "[2001:db8::1",
            "[2001:db8::1]:",
            "[2001:db8::1]:4242x",
            "[2001:db8::1]garbage",
            "[198.51.100.1]",
        ] {
            assert_eq!(resolve(&proxies, Some("10.0.0.1"), value), None, "{value}");
        }
    }

    #[test]
    fn an_unreadable_entry_leaves_the_client_unknown() {
        let proxies = TrustedProxies::new().nearest(1);
        assert_eq!(resolve(&proxies, Some("10.0.0.1"), "unknown"), None);
        assert_eq!(resolve(&proxies, Some("10.0.0.1"), "_hidden"), None);
        assert_eq!(resolve(&proxies, Some("10.0.0.1"), "<script>"), None);
    }

    #[test]
    fn an_unreadable_entry_beyond_the_client_is_never_read() {
        let proxies = TrustedProxies::new().nearest(1);
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "garbage, 198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn empty_entries_are_skipped() {
        let proxies = TrustedProxies::new().nearest(1);
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "198.51.100.1, ,"),
            Some(ip("198.51.100.1"))
        );
    }

    // -- the Forwarded header --

    #[test]
    fn the_forwarded_header_reads_the_for_parameter() {
        let proxies = TrustedProxies::new()
            .networks(["10.0.0.0/8"])
            .header(ForwardedHeader::Forwarded);
        let map = headers(&[(
            "forwarded",
            "For=198.51.100.1;proto=https, for=\"[2001:db8::1]:443\";by=10.0.0.1",
        )]);
        // The last element is the hop nearest to the application, and it is
        // not trusted.
        assert_eq!(
            proxies.client_ip(Some(ip("10.0.0.1")), &map),
            Some(ip("2001:db8::1"))
        );

        let map = headers(&[("forwarded", "for=198.51.100.1;proto=https, for=10.0.0.3")]);
        assert_eq!(
            proxies.client_ip(Some(ip("10.0.0.1")), &map),
            Some(ip("198.51.100.1"))
        );
    }

    #[test]
    fn forwarded_quoted_strings_may_contain_the_delimiters() {
        let proxies = TrustedProxies::new()
            .nearest(1)
            .header(ForwardedHeader::Forwarded);
        let resolve = |value: &str| {
            let map = headers(&[("forwarded", value)]);
            proxies.client_ip(Some(ip("10.0.0.1")), &map)
        };

        // A comma inside a quoted extension does not start a new element.
        assert_eq!(
            resolve("for=198.51.100.1;ext=\",for=203.0.113.9\""),
            Some(ip("198.51.100.1"))
        );
        // Nor does a semicolon start a new parameter.
        assert_eq!(
            resolve("for=\"[2001:db8::1]\";ext=\"a;for=203.0.113.9\""),
            Some(ip("2001:db8::1"))
        );
        // An escaped quote does not end the quoted string.
        assert_eq!(
            resolve("for=\"[2001:db8::1]:80\";ext=\"x\\\"y,for=203.0.113.9\""),
            Some(ip("2001:db8::1"))
        );
        // Escapes inside the address are unescaped.
        assert_eq!(resolve("for=\"198.51.\\100.1\""), Some(ip("198.51.100.1")));
    }

    #[test]
    fn a_forwarded_value_with_an_unclosed_quote_leaves_the_client_unknown() {
        let proxies = TrustedProxies::new()
            .nearest(1)
            .header(ForwardedHeader::Forwarded);
        for value in ["for=\"198.51.100.1", "for=\"198.51.100.1\\\""] {
            let map = headers(&[("forwarded", value)]);
            assert_eq!(proxies.client_ip(Some(ip("10.0.0.1")), &map), None);
        }
    }

    #[test]
    fn a_forwarded_element_without_an_address_leaves_the_client_unknown() {
        let proxies = TrustedProxies::new()
            .nearest(1)
            .header(ForwardedHeader::Forwarded);
        for value in ["proto=https", "for=unknown", "for=_hidden"] {
            let headers = headers(&[("forwarded", value)]);
            assert_eq!(proxies.client_ip(Some(ip("10.0.0.1")), &headers), None);
        }
    }

    #[test]
    fn the_forwarded_header_ignores_x_forwarded_for() {
        let proxies = TrustedProxies::new()
            .nearest(1)
            .header(ForwardedHeader::Forwarded);
        assert_eq!(
            resolve(&proxies, Some("10.0.0.1"), "198.51.100.1"),
            Some(ip("10.0.0.1"))
        );
    }

    // -- a single-address header --

    #[test]
    fn a_single_header_is_the_client_behind_a_trusted_peer() {
        let proxies =
            TrustedProxies::new()
                .networks(["10.0.0.0/8"])
                .header(ForwardedHeader::Single(HeaderName::from_static(
                    "cf-connecting-ip",
                )));
        let headers = headers(&[
            ("cf-connecting-ip", "198.51.100.1"),
            ("x-forwarded-for", "1.1.1.1, 198.51.100.1, 192.0.2.7"),
        ]);
        assert_eq!(
            proxies.client_ip(Some(ip("10.0.0.1")), &headers),
            Some(ip("198.51.100.1"))
        );
        // An untrusted peer is the client itself.
        assert_eq!(
            proxies.client_ip(Some(ip("203.0.113.9")), &headers),
            Some(ip("203.0.113.9"))
        );
        // A trusted peer that did not set the header leaves the client
        // unknown.
        assert_eq!(
            proxies.client_ip(Some(ip("10.0.0.1")), &HeaderMap::new()),
            None
        );
    }

    #[test]
    fn a_single_header_with_more_than_one_address_is_ambiguous() {
        let proxies = TrustedProxies::new()
            .nearest(1)
            .header(ForwardedHeader::Single(HeaderName::from_static(
                "x-real-ip",
            )));
        // Two fields, or one field listing two addresses.
        let map = headers(&[("x-real-ip", "1.1.1.1"), ("x-real-ip", "198.51.100.1")]);
        assert_eq!(proxies.client_ip(Some(ip("10.0.0.1")), &map), None);
        let map = headers(&[("x-real-ip", "1.1.1.1, 198.51.100.1")]);
        assert_eq!(proxies.client_ip(Some(ip("10.0.0.1")), &map), None);
    }
}

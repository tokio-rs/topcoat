use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use http::{HeaderMap, HeaderName, header};
use ipnet::{IpNet, Ipv4Net, Ipv6Net};

use crate::{RemoteAddr, request::Request};

/// The client's address as resolved when the request arrived, stored on the
/// request context of every dispatch.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientIp(pub(crate) Option<IpAddr>);

/// Configures which reverse proxies can report the client's IP address.
///
/// Behind a reverse proxy, the connection address belongs to the proxy.
/// The proxy reports the client's address in a header. Topcoat reads that
/// header only from trusted proxies because clients can also supply it.
///
/// Prefer [`networks`](Self::networks) when the proxies' addresses are known.
/// Otherwise, [`nearest`](Self::nearest) trusts a fixed number of proxy hops.
/// Use a hop count only when clients cannot bypass any of those proxies.
///
/// Register this configuration with
/// [`RouterBuilder::trusted_proxies`](crate::RouterBuilder::trusted_proxies),
/// then call [`client_ip`](crate::request::client_ip) to read the client's
/// address. By default, Topcoat trusts no proxies and uses the address of
/// the direct connection.
///
/// Each trusted proxy must update the configured [header](Self::header) with
/// its peer's address. Passing a client-supplied header through unchanged
/// lets the client fake its address.
///
/// # Examples
///
/// Trust proxies on a private network:
///
/// ```rust
/// use topcoat::router::{Router, TrustedProxies};
///
/// let router = Router::builder()
///     .trusted_proxies(TrustedProxies::new().networks(["10.0.0.0/8"]))
///     .build();
/// ```
///
/// Trust one load balancer whose IP address may change. Use this only when
/// every request must pass through that load balancer:
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
    networks: Vec<IpNet>,
    nearest: usize,
    header: ForwardedHeader,
}

impl TrustedProxies {
    /// Creates a configuration that trusts no proxies.
    ///
    /// Until you add trusted proxies with [`networks`](Self::networks) or
    /// [`nearest`](Self::nearest), [`client_ip`](crate::request::client_ip)
    /// uses the IP address of the direct connection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            networks: Vec::new(),
            nearest: 0,
            header: ForwardedHeader::XForwardedFor,
        }
    }

    /// Adds IP addresses or networks to the list of trusted proxies.
    ///
    /// Pass addresses such as `"127.0.0.1"` or networks in CIDR notation such
    /// as `"10.0.0.0/8"` and `"fd00::/8"`. You can also pass [`IpNet`] or
    /// [`IpAddr`] values. See [`IntoIpNet`] for the accepted types.
    ///
    /// IPv4 and IPv6 networks are matched separately. IPv4-mapped IPv6
    /// addresses, such as `::ffff:10.0.0.1`, are treated as IPv4 addresses.
    /// Networks within the mapped IPv6 range are also converted to IPv4.
    ///
    /// # Panics
    ///
    /// Panics if a string is not an address or a network in CIDR notation.
    #[must_use]
    #[track_caller]
    pub fn networks<I>(mut self, networks: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoIpNet,
    {
        /// The prefix length of the `::ffff:0:0/96` block holding the mapped
        /// addresses; a longer prefix denotes a network within it.
        const MAPPED_PREFIX: u8 = 96;

        for network in networks {
            let network = match network.into_ip_net() {
                // A network of mapped addresses is the IPv4 network it maps
                // to, the form addresses are compared in.
                IpNet::V6(v6) if v6.prefix_len() >= MAPPED_PREFIX => match v6.addr().to_canonical()
                {
                    IpAddr::V4(addr) => Ipv4Net::new(addr, v6.prefix_len() - MAPPED_PREFIX)
                        .map_or(IpNet::V6(v6), IpNet::V4),
                    IpAddr::V6(_) => IpNet::V6(v6),
                },
                network => network,
            };
            self.networks.push(network);
        }
        self
    }

    /// Trusts the `count` proxies closest to the application, regardless of
    /// their IP addresses.
    ///
    /// The proxy that connects directly to the application counts as the
    /// first. Further proxies are counted from right to left in the
    /// forwarding header. Use this when a proxy's IP address may change, or
    /// when it connects over a Unix socket and has no IP address.
    ///
    /// Every request must pass through all `count` proxies. For example, a
    /// CDN followed by a load balancer needs a count of two. If a client can
    /// connect directly to the load balancer, it occupies the second trusted
    /// hop and can fake its address through the header.
    ///
    /// Prefer [`networks`](Self::networks) when the proxies' addresses are known.
    #[must_use]
    pub fn nearest(mut self, count: usize) -> Self {
        self.nearest = count;
        self
    }

    /// Selects the header used to read the client's IP address.
    ///
    /// Defaults to [`ForwardedHeader::XForwardedFor`]. Choose the header your
    /// proxy updates. Topcoat ignores the other headers because they may
    /// contain values supplied by the client.
    #[must_use]
    pub fn header(mut self, header: ForwardedHeader) -> Self {
        self.header = header;
        self
    }

    /// Resolves the client address of `request` from the connection it
    /// arrived on and the forwarding headers it carries.
    pub(crate) fn resolve(&self, request: &Request) -> Option<IpAddr> {
        let remote = request
            .extensions()
            .get::<RemoteAddr>()
            .map(|remote| remote.0.ip());
        self.client_ip(remote, request.headers())
    }

    /// Resolves the client address of a request that arrived from `remote`
    /// with `headers`.
    ///
    /// Walks outward from the direct connection and returns the first
    /// untrusted address. If every address is trusted, returns the farthest.
    /// An unreadable address stops resolution unless its position is trusted,
    /// in which case the walk continues.
    pub(crate) fn client_ip(&self, remote: Option<IpAddr>, headers: &HeaderMap) -> Option<IpAddr> {
        let remote = remote.map(|ip| ip.to_canonical());
        if !self.is_trusted(remote, 0) {
            return remote;
        }

        // The lists are split as bytes and each entry is decoded on its own,
        // so a byte outside ASCII that a client put in its entry spoils that
        // entry alone, not the address a proxy appended after it.
        let values = headers.get_all(self.header.name()).iter();

        match self.header {
            ForwardedHeader::Single(_) => {
                // Exactly one field holding exactly one address; any other
                // shape is ambiguous, so it names none.
                let mut values = values;
                let value = values.next()?;
                if values.next().is_some() {
                    return None;
                }
                parse_node(value.to_str().ok()?.trim())
            }
            ForwardedHeader::Forwarded => {
                // A quote left open would swallow whatever a proxy appends
                // after it, so a field with one is malformed as a whole.
                let fields: Vec<&[u8]> = values
                    .map(|value| {
                        let field = value.as_bytes();
                        is_well_quoted(field).then_some(field)
                    })
                    .collect::<Option<_>>()?;
                // An element's quoted strings may contain the list and
                // parameter delimiters, so the split reads front to back with
                // the quoting in mind, and the walk goes over the addresses
                // from the back.
                let addresses: Vec<Option<IpAddr>> = fields
                    .into_iter()
                    .flat_map(|field| {
                        split_quoted(field, b',')
                            .filter(|element| !element.trim_ascii().is_empty())
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
                        .as_bytes()
                        .split(|&byte| byte == b',')
                        .map(<[u8]>::trim_ascii)
                        .filter(|entry| !entry.is_empty())
                        .rev()
                });
                self.walk(
                    remote,
                    entries.map(|entry| str::from_utf8(entry).ok().and_then(parse_node)),
                )
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
            let hop = hop + 1;
            // A hop trusted by position needs no readable address; any other
            // hop must name one to be checked against the networks.
            if hop >= self.nearest {
                let ip = address?;
                if !self.is_trusted(Some(ip), hop) {
                    return Some(ip);
                }
            }
            farthest = address;
        }
        farthest
    }

    /// Whether the proxy at `addr`, `hop` positions out from the
    /// application, is trusted.
    fn is_trusted(&self, addr: Option<IpAddr>, hop: usize) -> bool {
        hop < self.nearest
            || addr.is_some_and(|ip| self.networks.iter().any(|network| network.contains(&ip)))
    }
}

/// Trusts no proxy.
impl Default for TrustedProxies {
    fn default() -> Self {
        Self::new()
    }
}

/// The HTTP header used to read the client's IP address from a trusted proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardedHeader {
    /// A comma-separated list of IP addresses in `X-Forwarded-For`.
    ///
    /// Each proxy appends the address it received the request from, so the
    /// client's address comes first, followed by the proxies' addresses.
    /// This is the default header.
    XForwardedFor,
    /// The `Forwarded` header (RFC 7239).
    ///
    /// Each proxy adds an entry with a `for=` parameter containing the address
    /// it received the request from. Topcoat reads these parameters in the
    /// same order as the addresses in `X-Forwarded-For`.
    Forwarded,
    /// A header containing only the client's IP address, such as `CF-Connecting-IP`
    /// or `True-Client-IP`.
    ///
    /// If the direct connection is trusted, Topcoat uses this header's value
    /// as the client's address. The header must appear exactly once and
    /// contain exactly one address. Otherwise,
    /// [`client_ip`](crate::request::client_ip) returns `None`.
    ///
    /// The proxy connecting to the application must set this header itself
    /// or verify that it came from another trusted proxy. If a load balancer
    /// simply passes the header through, a client may be able to supply a
    /// fake address.
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

/// A value that can be passed to [`TrustedProxies::networks`].
///
/// Strings can contain a network in CIDR notation (`"10.0.0.0/8"`) or a
/// single IP address (`"10.0.0.1"`). Invalid strings cause a panic. An IP
/// address becomes a network containing only that address. Network values
/// are used as they are.
pub trait IntoIpNet {
    /// Converts the value into a network.
    ///
    /// # Panics
    ///
    /// Panics if the value is a string that is neither an address nor a
    /// network in CIDR notation.
    #[track_caller]
    fn into_ip_net(self) -> IpNet;
}

impl IntoIpNet for &str {
    #[track_caller]
    fn into_ip_net(self) -> IpNet {
        self.parse::<IpNet>()
            .or_else(|_| self.parse::<IpAddr>().map(IpNet::from))
            .unwrap_or_else(|_| panic!("invalid trusted proxy network `{self}`"))
    }
}

impl IntoIpNet for String {
    #[track_caller]
    fn into_ip_net(self) -> IpNet {
        self.as_str().into_ip_net()
    }
}

impl IntoIpNet for IpNet {
    fn into_ip_net(self) -> IpNet {
        self
    }
}

impl IntoIpNet for Ipv4Net {
    fn into_ip_net(self) -> IpNet {
        IpNet::V4(self)
    }
}

impl IntoIpNet for Ipv6Net {
    fn into_ip_net(self) -> IpNet {
        IpNet::V6(self)
    }
}

impl IntoIpNet for IpAddr {
    fn into_ip_net(self) -> IpNet {
        IpNet::from(self)
    }
}

/// A reference to any accepted value, so a stored list can be passed by
/// reference.
impl<T> IntoIpNet for &T
where
    T: IntoIpNet + Clone,
{
    #[track_caller]
    fn into_ip_net(self) -> IpNet {
        self.clone().into_ip_net()
    }
}

/// Reads an IP address from a forwarding header value, ignoring any port.
/// IPv6 addresses with a port must be enclosed in brackets.
/// Returns `None` for `unknown`, hidden addresses such as `_client`, or
/// invalid values.
fn parse_node(value: &str) -> Option<IpAddr> {
    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip.to_canonical());
    }
    let (ip, port) = if let Some(rest) = value.strip_prefix('[') {
        let (ip, rest) = rest.split_once(']')?;
        let ip = IpAddr::V6(ip.parse::<Ipv6Addr>().ok()?);
        if rest.is_empty() {
            return Some(ip.to_canonical());
        }
        (ip, rest.strip_prefix(':')?)
    } else {
        let (ip, port) = value.rsplit_once(':')?;
        (IpAddr::V4(ip.parse::<Ipv4Addr>().ok()?), port)
    };
    is_node_port(port).then(|| ip.to_canonical())
}

/// Checks whether a forwarding header's port is one to five digits or a
/// hidden value starting with `_`. Hidden values must contain at least one
/// ASCII letter, digit, `.`, `_`, or `-` after the leading `_`, and no other
/// characters.
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

/// Reads the `for` value from one entry in a `Forwarded` header and removes
/// its surrounding quotes and backslash escapes.
/// Returns `None` if `for` is missing or repeated, a parameter has no `=` or
/// a byte outside ASCII, or the value has an unclosed quote or an incomplete
/// escape.
fn forwarded_for(element: &[u8]) -> Option<Cow<'_, str>> {
    let mut found = None;
    for pair in split_quoted(element, b';') {
        let pair = str::from_utf8(pair).ok()?;
        if pair.trim().is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=')?;
        if name.trim().eq_ignore_ascii_case("for") {
            // Reject duplicate `for` parameters because we cannot tell
            // which address to use.
            if found.is_some() {
                return None;
            }
            found = Some(unquote(value.trim())?);
        }
    }
    found
}

/// Tracks quoted strings while scanning a header value byte by byte.
///
/// A backslash inside a quoted string escapes the next byte, so an escaped
/// quote does not end the string.
#[derive(Default)]
struct QuoteState {
    /// Whether the scan is inside a quoted string.
    quoted: bool,
    /// Whether the previous byte was a backslash inside a quoted string.
    escaped: bool,
}

impl QuoteState {
    /// Advances past `byte`, returning whether it is plain text outside any
    /// quoted string, so delimiters found there count.
    fn feed(&mut self, byte: u8) -> bool {
        if self.escaped {
            self.escaped = false;
        } else if self.quoted {
            match byte {
                b'\\' => self.escaped = true,
                b'"' => self.quoted = false,
                _ => {}
            }
        } else if byte == b'"' {
            self.quoted = true;
        } else {
            return true;
        }
        false
    }

    /// Whether every quoted string has been closed and no escape is pending.
    fn is_closed(&self) -> bool {
        !self.quoted && !self.escaped
    }
}

/// Checks that every opening quote has a closing quote and every backslash
/// inside a quoted string has a character after it.
fn is_well_quoted(value: &[u8]) -> bool {
    let mut state = QuoteState::default();
    for &byte in value {
        state.feed(byte);
    }
    state.is_closed()
}

/// Splits `value` at each `delimiter`, keeping quoted strings together.
/// Check the quotes with [`is_well_quoted`] before calling this function.
fn split_quoted(value: &[u8], delimiter: u8) -> impl Iterator<Item = &[u8]> {
    let mut state = QuoteState::default();
    value.split(move |&byte| state.feed(byte) && byte == delimiter)
}

/// Removes surrounding quotes and replaces each backslash escape with the
/// character after it. Returns unquoted values unchanged.
/// Returns `None` if a quoted value has no closing quote or ends with an
/// incomplete escape.
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
    fn networks_accept_parsed_values() {
        let network: IpNet = "10.0.0.0/8".parse().unwrap();
        let proxies = TrustedProxies::new()
            .networks([network])
            .networks([ip("192.0.2.7")]);
        for peer in ["10.1.2.3", "192.0.2.7"] {
            assert_eq!(
                resolve(&proxies, Some(peer), "198.51.100.1"),
                Some(ip("198.51.100.1"))
            );
        }
        assert_eq!(
            resolve(&proxies, Some("192.0.2.8"), "198.51.100.1"),
            Some(ip("192.0.2.8"))
        );
    }

    #[test]
    fn networks_accept_references() {
        // A list read from configuration, passed without moving it.
        let strings = vec![String::from("10.0.0.0/8")];
        let addresses = [ip("192.0.2.7")];
        let proxies = TrustedProxies::new()
            .networks(&strings)
            .networks(addresses.iter());
        for peer in ["10.1.2.3", "192.0.2.7"] {
            assert_eq!(
                resolve(&proxies, Some(peer), "198.51.100.1"),
                Some(ip("198.51.100.1"))
            );
        }
        assert_eq!(
            resolve(&proxies, Some("192.0.2.8"), "198.51.100.1"),
            Some(ip("192.0.2.8"))
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
    fn a_hop_trusted_by_position_needs_no_readable_address() {
        // The load balancer names itself in a form the header cannot carry
        // as an address, as RFC 7239 allows for intermediate hops.
        let proxies = TrustedProxies::new().nearest(2);
        for value in ["198.51.100.1, unknown", "198.51.100.1, _lb1"] {
            assert_eq!(
                resolve(&proxies, Some("203.0.113.9"), value),
                Some(ip("198.51.100.1")),
                "{value}"
            );
        }
        // The client itself must still be readable.
        assert_eq!(resolve(&proxies, Some("203.0.113.9"), "unknown"), None);
        assert_eq!(
            resolve(&proxies, Some("203.0.113.9"), "unknown, 10.0.0.3"),
            None
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

    #[test]
    fn a_byte_outside_ascii_spoils_its_entry_only() {
        let proxies = TrustedProxies::new().nearest(1);
        let resolve = |value: &[u8]| {
            let mut map = HeaderMap::new();
            map.append("x-forwarded-for", HeaderValue::from_bytes(value).unwrap());
            proxies.client_ip(Some(ip("10.0.0.1")), &map)
        };

        // The client sent an entry the header cannot decode, and the proxy
        // appended the real address after it.
        assert_eq!(resolve(b"\xff\xfe, 198.51.100.1"), Some(ip("198.51.100.1")));
        // The same byte within the entry the walk reaches leaves the client
        // unknown.
        assert_eq!(resolve(b"198.51.100.1\xff"), None);
        assert_eq!(resolve(b"\xff"), None);
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
        for value in [
            "for=\"198.51.100.1",
            "for=\"198.51.100.1\\\"",
            // The client left a quote open and the proxy appended the real
            // address after it; the client's address must not win.
            "for=203.0.113.9;ext=\", for=198.51.100.1",
        ] {
            let map = headers(&[("forwarded", value)]);
            assert_eq!(
                proxies.client_ip(Some(ip("10.0.0.1")), &map),
                None,
                "{value}"
            );
        }

        // The same with the proxy adding a field of its own: a malformed
        // field spoils the whole header.
        let map = headers(&[
            ("forwarded", "for=203.0.113.9;ext=\""),
            ("forwarded", "for=198.51.100.1"),
        ]);
        assert_eq!(proxies.client_ip(Some(ip("10.0.0.1")), &map), None);
    }

    #[test]
    fn a_forwarded_byte_outside_ascii_spoils_its_element_only() {
        let proxies = TrustedProxies::new()
            .nearest(1)
            .header(ForwardedHeader::Forwarded);
        let resolve = |value: &[u8]| {
            let mut map = HeaderMap::new();
            map.append("forwarded", HeaderValue::from_bytes(value).unwrap());
            proxies.client_ip(Some(ip("10.0.0.1")), &map)
        };

        // The proxy appended its element after one the client sent.
        assert_eq!(
            resolve(b"for=\xff\xfe, for=198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
        // A quoted string may hold such a byte without ending early.
        assert_eq!(
            resolve(b"for=203.0.113.9;ext=\"\xff,for=1.1.1.1\", for=198.51.100.1"),
            Some(ip("198.51.100.1"))
        );
        // The element the walk reaches carries no address.
        assert_eq!(resolve(b"for=198.51.100.1;ext=\xff"), None);
    }

    #[test]
    fn a_forwarded_element_with_two_for_parameters_is_ambiguous() {
        let proxies = TrustedProxies::new()
            .nearest(1)
            .header(ForwardedHeader::Forwarded);
        let map = headers(&[("forwarded", "for=203.0.113.9;for=198.51.100.1")]);
        assert_eq!(proxies.client_ip(Some(ip("10.0.0.1")), &map), None);
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

    #[test]
    fn a_single_header_with_a_byte_outside_ascii_is_unreadable() {
        let proxies = TrustedProxies::new()
            .nearest(1)
            .header(ForwardedHeader::Single(HeaderName::from_static(
                "x-real-ip",
            )));
        let mut map = HeaderMap::new();
        map.append(
            "x-real-ip",
            HeaderValue::from_bytes(b"198.51.100.1\xff").unwrap(),
        );
        assert_eq!(proxies.client_ip(Some(ip("10.0.0.1")), &map), None);
    }
}

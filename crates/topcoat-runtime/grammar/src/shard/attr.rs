use syn::parse::{Parse, ParseStream};

/// The transport used to re-render a shard after hydration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardTransport {
    Http,
    WebSocket,
}

/// Arguments passed to the `#[shard]` attribute itself.
pub struct ShardAttr {
    transport: ShardTransport,
}

impl ShardAttr {
    #[must_use]
    pub fn transport(&self) -> ShardTransport {
        self.transport
    }
}

impl Parse for ShardAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self {
                transport: ShardTransport::Http,
            });
        }

        let option: syn::Ident = input.parse()?;
        if option != "ws" {
            return Err(syn::Error::new(
                option.span(),
                format!("unknown shard option `{option}`; expected `ws`"),
            ));
        }
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after shard option `ws`"));
        }

        Ok(Self {
            transport: ShardTransport::WebSocket,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<ShardAttr>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn accepts_no_option_or_ws() {
        assert_eq!(
            syn::parse_str::<ShardAttr>("").unwrap().transport(),
            ShardTransport::Http
        );
        assert_eq!(
            syn::parse_str::<ShardAttr>("ws").unwrap().transport(),
            ShardTransport::WebSocket
        );
    }

    #[test]
    fn rejects_unknown_and_trailing_options() {
        assert!(parse_err("sse").contains("unknown shard option `sse`"));
        assert!(parse_err("ws, sse").contains("unexpected tokens"));
        assert!(parse_err("ws trailing").contains("unexpected tokens"));
    }
}

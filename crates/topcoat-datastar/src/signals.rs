use std::ops::{Deref, DerefMut};

use serde::{Deserialize, de::DeserializeOwned};
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Body, FromRequest, Method, OptionalFromRequest, content::Json, error::bad_request, method,
    parse_query_params,
};

use crate::datastar_request;

/// Extracts the signals a Datastar action sends with a request.
///
/// A Datastar action includes the page's signals with every request (except
/// those prefixed with an underscore): GET requests carry them JSON-encoded in
/// the `datastar` query parameter, all other requests as a JSON body.
/// `Signals<T>` reads them from either place and deserializes them into `T`.
///
/// Wrap it in [`Option`] to also accept requests made without Datastar: the
/// extractor yields [`None`] when the request carries no `Datastar-Request`
/// header, and still reports an error when the signals are malformed.
///
/// # Examples
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use topcoat::{
///     Result,
///     datastar::{PatchSignals, Signals},
///     router::route,
/// };
///
/// #[derive(Deserialize, Serialize)]
/// struct Counter {
///     count: u64,
/// }
///
/// #[route(POST "/increment")]
/// async fn increment(Signals(counter): Signals<Counter>) -> Result<PatchSignals> {
///     PatchSignals::json(&Counter {
///         count: counter.count + 1,
///     })
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Signals<T>(pub T);

impl<T> From<T> for Signals<T> {
    fn from(signals: T) -> Self {
        Self(signals)
    }
}

impl<T> Deref for Signals<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Signals<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The query parameter Datastar sends signals in on GET requests.
#[derive(Deserialize)]
struct DatastarParam {
    datastar: String,
}

impl<T> FromRequest for Signals<T>
where
    T: DeserializeOwned,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        if method(cx) == Method::GET {
            let DatastarParam { datastar } = parse_query_params(cx).map_err(|error| {
                bad_request(format!(
                    "failed to read the `datastar` query parameter: {error}"
                ))
            })?;

            Ok(Self(Json::from_bytes(datastar.as_bytes())?.0))
        } else {
            let Json(signals) = <Json<T> as FromRequest>::from_request(cx, body).await?;
            Ok(Self(signals))
        }
    }
}

impl<T> OptionalFromRequest for Signals<T>
where
    T: DeserializeOwned,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Option<Self>> {
        if datastar_request(cx) {
            Ok(Some(<Self as FromRequest>::from_request(cx, body).await?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use http::Request;
    use http::header::CONTENT_TYPE;
    use serde_json::{Value, json};
    use topcoat_core::context::CxTestBuilder;
    use topcoat_router::error::BadRequestError;

    use super::*;
    use crate::header;

    fn cx_for(request: Request<()>) -> Cx {
        let (parts, ()) = request.into_parts();
        CxTestBuilder::new().request_context(parts).build()
    }

    #[tokio::test]
    async fn get_requests_carry_signals_in_the_query() {
        let cx = cx_for(
            Request::builder()
                .method(Method::GET)
                .uri("/counter?datastar=%7B%22count%22%3A1%7D")
                .body(())
                .unwrap(),
        );

        let Signals(signals) = <Signals<Value> as FromRequest>::from_request(&cx, Body::empty())
            .await
            .unwrap();
        assert_eq!(signals, json!({ "count": 1 }));
    }

    #[tokio::test]
    async fn get_requests_without_the_parameter_are_bad_requests() {
        let cx = cx_for(
            Request::builder()
                .method(Method::GET)
                .uri("/counter")
                .body(())
                .unwrap(),
        );

        let error = <Signals<Value> as FromRequest>::from_request(&cx, Body::empty())
            .await
            .expect_err("a missing parameter is rejected");
        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn other_requests_carry_signals_as_a_json_body() {
        let cx = cx_for(
            Request::builder()
                .method(Method::POST)
                .uri("/counter")
                .header(CONTENT_TYPE, "application/json")
                .body(())
                .unwrap(),
        );

        let Signals(signals) =
            <Signals<Value> as FromRequest>::from_request(&cx, Body::from(r#"{"count":2}"#))
                .await
                .unwrap();
        assert_eq!(signals, json!({ "count": 2 }));
    }

    #[tokio::test]
    async fn optional_extraction_requires_the_datastar_header() {
        let cx = cx_for(
            Request::builder()
                .method(Method::POST)
                .uri("/counter")
                .body(())
                .unwrap(),
        );

        let signals = <Signals<Value> as OptionalFromRequest>::from_request(&cx, Body::empty())
            .await
            .expect("a non-Datastar request is not an error");
        assert!(signals.is_none());

        let cx = cx_for(
            Request::builder()
                .method(Method::POST)
                .uri("/counter")
                .header(&header::DATASTAR_REQUEST, "true")
                .header(CONTENT_TYPE, "application/json")
                .body(())
                .unwrap(),
        );

        let signals = <Signals<Value> as OptionalFromRequest>::from_request(
            &cx,
            Body::from(r#"{"count":3}"#),
        )
        .await
        .expect("a valid body is not an error");
        assert_eq!(
            signals.expect("signals are present").0,
            json!({ "count": 3 })
        );
    }
}

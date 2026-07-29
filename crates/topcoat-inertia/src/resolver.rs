use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Value};
use topcoat_core::error::Result;
use web_time::{SystemTime, UNIX_EPOCH};

use crate::page::{OnceMetadata, Page};
use crate::prop::{InitialBehavior, MergeBehavior, OnceBehavior, Prop, ScrollBehavior};
use crate::{InertiaRequest, MergeIntent};

pub(crate) struct PropEntry<'cx> {
    pub(crate) path: String,
    pub(crate) prop: Prop<'cx>,
    pub(crate) shared: bool,
}

pub(crate) async fn resolve(
    component: String,
    url: String,
    version: Option<String>,
    entries: Vec<PropEntry<'_>>,
    errors: Value,
    request: &InertiaRequest,
) -> Result<Page> {
    let partial = request.is_partial_for(&component);
    let mut page = Page {
        component,
        props: Map::new(),
        url,
        version,
        encrypt_history: false,
        clear_history: false,
        preserve_fragment: false,
        deferred_props: BTreeMap::new(),
        merge_props: Vec::new(),
        prepend_props: Vec::new(),
        deep_merge_props: Vec::new(),
        match_props_on: Vec::new(),
        once_props: BTreeMap::new(),
        scroll_props: BTreeMap::new(),
        rescued_props: Vec::new(),
        shared_props: vec!["errors".to_owned()],
        flash: Map::new(),
    };

    insert_path(&mut page.props, "errors", errors)?;

    for entry in entries {
        validate_path(&entry.path)?;
        if entry.path.split('.').next() == Some("errors") {
            return Err(ResolveError::new("`errors` is a reserved Inertia prop").into());
        }
        entry.prop.validate(&entry.path)?;
        if entry.shared {
            let top = entry.path.split('.').next().unwrap_or_default().to_owned();
            push_unique(&mut page.shared_props, top);
        }

        let path = entry.path;
        let prop = entry.prop;
        if partial && !prop.always && !value_selected(request, &path) {
            continue;
        }

        let cached_once = prop.once.as_ref().is_some_and(|once| {
            request.is_inertia()
                && !once.fresh
                && request
                    .except_once()
                    .iter()
                    .any(|loaded| loaded == once.key.as_deref().unwrap_or(path.as_str()))
        });
        let explicitly_requested = partial && metadata_selected(request, &path);

        if cached_once && !explicitly_requested {
            if let Some(once) = &prop.once
                && metadata_selected_or_full(request, partial, &path)
            {
                record_once(&mut page, &path, once)?;
            }
            continue;
        }

        if !partial && prop.initial != InitialBehavior::Include {
            if let InitialBehavior::Deferred { group } = &prop.initial {
                page.deferred_props
                    .entry(group.to_string())
                    .or_default()
                    .push(path.clone());
                record_merge(&mut page, request, partial, &path, prop.merge.as_ref());
                if let Some(scroll) = &prop.scroll {
                    record_scroll_merge(&mut page, request, partial, &path, scroll);
                }
                if let Some(once) = &prop.once {
                    record_once(&mut page, &path, once)?;
                }
            }
            continue;
        }

        let value = match prop.source.resolve().await {
            Ok(value) => value,
            Err(_error) if prop.rescue => {
                page.rescued_props.push(path);
                continue;
            }
            Err(error) => return Err(error),
        };

        record_merge(&mut page, request, partial, &path, prop.merge.as_ref());
        if let Some(scroll) = prop.scroll {
            record_scroll(&mut page, request, partial, &path, scroll);
        }
        if let Some(once) = &prop.once
            && metadata_selected_or_full(request, partial, &path)
        {
            record_once(&mut page, &path, once)?;
        }
        insert_path(&mut page.props, &path, value)?;
    }

    Ok(page)
}

fn value_selected(request: &InertiaRequest, path: &str) -> bool {
    let included = request.only().is_none_or(|only| {
        only.iter()
            .any(|selected| related_in_either_direction(path, selected))
    });
    let excluded = request.except().is_some_and(|except| {
        except
            .iter()
            .any(|selected| path == selected || is_descendant(path, selected))
    });
    included && !excluded
}

fn metadata_selected_or_full(request: &InertiaRequest, partial: bool, path: &str) -> bool {
    !partial || metadata_selected(request, path)
}

fn metadata_selected(request: &InertiaRequest, path: &str) -> bool {
    let included = request.only().is_none_or(|only| {
        only.iter()
            .any(|selected| path == selected || is_descendant(path, selected))
    });
    let excluded = request.except().is_some_and(|except| {
        except
            .iter()
            .any(|selected| path == selected || is_descendant(path, selected))
    });
    included && !excluded
}

fn related_in_either_direction(left: &str, right: &str) -> bool {
    left == right || is_descendant(left, right) || is_descendant(right, left)
}

fn is_descendant(path: &str, ancestor: &str) -> bool {
    path.strip_prefix(ancestor)
        .is_some_and(|suffix| suffix.starts_with('.'))
}

fn record_merge(
    page: &mut Page,
    request: &InertiaRequest,
    partial: bool,
    path: &str,
    merge: Option<&MergeBehavior>,
) {
    let Some(merge) = merge else { return };
    if request.reset().iter().any(|reset| reset == path)
        || !metadata_selected_or_full(request, partial, path)
    {
        return;
    }
    if merge.deep {
        push_unique(&mut page.deep_merge_props, path.to_owned());
    } else {
        if merge.append_root {
            push_unique(&mut page.merge_props, path.to_owned());
        }
        if merge.prepend_root {
            push_unique(&mut page.prepend_props, path.to_owned());
        }
        for child in &merge.append {
            push_unique(&mut page.merge_props, format!("{path}.{child}"));
        }
        for child in &merge.prepend {
            push_unique(&mut page.prepend_props, format!("{path}.{child}"));
        }
    }
    for child in &merge.match_on {
        push_unique(&mut page.match_props_on, format!("{path}.{child}"));
    }
}

fn record_scroll_merge(
    page: &mut Page,
    request: &InertiaRequest,
    partial: bool,
    path: &str,
    scroll: &ScrollBehavior,
) {
    if request.reset().iter().any(|reset| reset == path)
        || !metadata_selected_or_full(request, partial, path)
    {
        return;
    }
    let merge_path = scroll
        .wrapper
        .as_ref()
        .map_or_else(|| path.to_owned(), |wrapper| format!("{path}.{wrapper}"));
    match request.scroll_intent() {
        MergeIntent::Append => push_unique(&mut page.merge_props, merge_path),
        MergeIntent::Prepend => push_unique(&mut page.prepend_props, merge_path),
    }
}

fn record_scroll(
    page: &mut Page,
    request: &InertiaRequest,
    partial: bool,
    path: &str,
    mut scroll: ScrollBehavior,
) {
    record_scroll_merge(page, request, partial, path, &scroll);
    scroll
        .metadata
        .set_reset(request.reset().iter().any(|reset| reset == path));
    page.scroll_props.insert(path.to_owned(), scroll.metadata);
}

fn record_once(page: &mut Page, path: &str, once: &OnceBehavior) -> Result<()> {
    let expires_at = once
        .expires
        .map(|duration| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ResolveError::new("system clock is before the Unix epoch"))?;
            u64::try_from(now.as_millis().saturating_add(duration.as_millis()))
                .map_err(|_| ResolveError::new("once prop expiry is too large"))
        })
        .transpose()?;
    page.once_props.insert(
        once.key.clone().unwrap_or_else(|| path.to_owned()),
        OnceMetadata {
            prop: path.to_owned(),
            expires_at,
        },
    );
    Ok(())
}

fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() || path.split('.').any(str::is_empty) {
        return Err(ResolveError::new("Inertia prop paths cannot contain empty segments").into());
    }
    Ok(())
}

fn insert_path(props: &mut Map<String, Value>, path: &str, value: Value) -> Result<()> {
    let mut segments = path.split('.');
    let first = segments
        .next()
        .ok_or_else(|| ResolveError::new("Inertia prop path cannot be empty"))?;
    let rest = segments.collect::<Vec<_>>();
    if rest.is_empty() {
        props.insert(first.to_owned(), value);
        return Ok(());
    }
    let target = props.entry(first.to_owned()).or_insert(Value::Null);
    insert_segments(target, &rest, value, path)
}

fn insert_segments(target: &mut Value, segments: &[&str], value: Value, path: &str) -> Result<()> {
    let Some((segment, rest)) = segments.split_first() else {
        *target = value;
        return Ok(());
    };
    let numeric = segment.parse::<usize>().ok();
    if let Some(index) = numeric {
        if target.is_null() {
            *target = Value::Array(Vec::new());
        }
        let Some(array) = target.as_array_mut() else {
            return Err(ResolveError::collision(path).into());
        };
        if array.len() <= index {
            array.resize(index + 1, Value::Null);
        }
        insert_segments(&mut array[index], rest, value, path)
    } else {
        if target.is_null() {
            *target = Value::Object(Map::new());
        }
        let Some(object) = target.as_object_mut() else {
            return Err(ResolveError::collision(path).into());
        };
        let child = object.entry((*segment).to_owned()).or_insert(Value::Null);
        insert_segments(child, rest, value, path)
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[derive(Debug)]
struct ResolveError(String);

impl ResolveError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    fn collision(path: &str) -> Self {
        Self(format!(
            "Inertia prop path `{path}` collides with an incompatible value"
        ))
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ResolveError {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use http::{HeaderMap, HeaderValue};
    use serde_json::json;
    use topcoat_core::error::Result;

    use super::*;
    use crate::{ScrollMetadata, defer, header, lazy, merge, optional, scroll, value};

    fn request(headers: &[(&http::HeaderName, &str)]) -> InertiaRequest {
        let mut map = HeaderMap::new();
        for (name, value) in headers {
            map.insert(*name, HeaderValue::from_str(value).unwrap());
        }
        InertiaRequest::from_headers(&map)
    }

    fn entry(path: &str, prop: Prop<'static>) -> PropEntry<'static> {
        PropEntry {
            path: path.to_owned(),
            prop,
            shared: false,
        }
    }

    async fn page(entries: Vec<PropEntry<'_>>, request: &InertiaRequest) -> Result<Page> {
        resolve(
            "Users/Index".to_owned(),
            "/users?page=2".to_owned(),
            Some("v1".to_owned()),
            entries,
            json!({}),
            request,
        )
        .await
    }

    #[tokio::test]
    async fn initial_response_skips_optional_and_deferred_futures() -> Result<()> {
        let optional_polled = Arc::new(AtomicBool::new(false));
        let deferred_polled = Arc::new(AtomicBool::new(false));
        let optional_flag = optional_polled.clone();
        let deferred_flag = deferred_polled.clone();

        let page = page(
            vec![
                entry(
                    "audit",
                    optional(async move {
                        optional_flag.store(true, Ordering::SeqCst);
                        Ok::<_, topcoat_core::error::Error>(1)
                    }),
                ),
                entry(
                    "stats",
                    defer(async move {
                        deferred_flag.store(true, Ordering::SeqCst);
                        Ok::<_, topcoat_core::error::Error>(2)
                    })
                    .group("dashboard")
                    .merge()
                    .once(),
                ),
            ],
            &InertiaRequest::default(),
        )
        .await?;

        assert!(!optional_polled.load(Ordering::SeqCst));
        assert!(!deferred_polled.load(Ordering::SeqCst));
        assert_eq!(page.props, serde_json::from_value(json!({"errors": {}}))?);
        assert_eq!(page.deferred_props["dashboard"], ["stats"]);
        assert_eq!(page.merge_props, ["stats"]);
        assert_eq!(page.once_props["stats"].prop, "stats");
        Ok(())
    }

    #[tokio::test]
    async fn partial_filters_match_ancestors_and_except_wins() -> Result<()> {
        let request = request(&[
            (&header::X_INERTIA, "true"),
            (&header::X_INERTIA_PARTIAL_COMPONENT, "Users/Index"),
            (&header::X_INERTIA_PARTIAL_DATA, "user.name,posts"),
            (&header::X_INERTIA_PARTIAL_EXCEPT, "posts"),
        ]);
        let page = page(
            vec![
                entry("user", value(json!({"name": "Ada", "role": "admin"}))),
                entry("posts", value([1, 2])),
                entry("locale", value("en").always()),
            ],
            &request,
        )
        .await?;

        assert_eq!(
            page.props,
            serde_json::from_value(json!({
                "errors": {},
                "user": {"name": "Ada", "role": "admin"},
                "locale": "en"
            }))?
        );
        assert!(page.deferred_props.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn mismatched_partial_component_resolves_a_full_page() -> Result<()> {
        let request = request(&[
            (&header::X_INERTIA, "true"),
            (&header::X_INERTIA_PARTIAL_COMPONENT, "Dashboard"),
            (&header::X_INERTIA_PARTIAL_DATA, "users"),
        ]);
        let page = page(
            vec![entry("users", value([1])), entry("other", value(2))],
            &request,
        )
        .await?;

        assert_eq!(page.props["users"], json!([1]));
        assert_eq!(page.props["other"], json!(2));
        Ok(())
    }

    #[tokio::test]
    async fn partial_metadata_requires_a_direct_or_descendant_match() -> Result<()> {
        let request = request(&[
            (&header::X_INERTIA, "true"),
            (&header::X_INERTIA_PARTIAL_COMPONENT, "Users/Index"),
            (&header::X_INERTIA_PARTIAL_DATA, "dashboard.stats"),
        ]);
        let page = page(
            vec![entry(
                "dashboard",
                merge(json!({"stats": [1]})).match_on("id"),
            )],
            &request,
        )
        .await?;

        assert!(page.props.contains_key("dashboard"));
        assert!(page.merge_props.is_empty());
        assert!(page.match_props_on.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn deferred_partial_is_resolved_without_reemitting_its_group() -> Result<()> {
        let request = request(&[
            (&header::X_INERTIA, "true"),
            (&header::X_INERTIA_PARTIAL_COMPONENT, "Users/Index"),
            (&header::X_INERTIA_PARTIAL_DATA, "stats"),
        ]);
        let page = page(
            vec![entry(
                "stats",
                defer(async { Ok::<_, topcoat_core::error::Error>(json!({"total": 3})) }).rescue(),
            )],
            &request,
        )
        .await?;

        assert_eq!(page.props["stats"], json!({"total": 3}));
        assert!(page.deferred_props.is_empty());
        assert!(page.rescued_props.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn rescued_deferred_errors_are_reported_as_metadata() -> Result<()> {
        let request = request(&[
            (&header::X_INERTIA, "true"),
            (&header::X_INERTIA_PARTIAL_COMPONENT, "Users/Index"),
            (&header::X_INERTIA_PARTIAL_DATA, "stats"),
        ]);
        let page = page(
            vec![entry(
                "stats",
                lazy(async { Err::<Value, _>(ResolveError::new("unavailable").into()) })
                    .defer()
                    .rescue(),
            )],
            &request,
        )
        .await?;

        assert!(!page.props.contains_key("stats"));
        assert_eq!(page.rescued_props, ["stats"]);
        Ok(())
    }

    #[tokio::test]
    async fn once_cache_exclusion_composes_with_deferred_props() -> Result<()> {
        let request = request(&[
            (&header::X_INERTIA, "true"),
            (&header::X_INERTIA_EXCEPT_ONCE_PROPS, "navigation"),
        ]);
        let page = page(
            vec![entry(
                "nav",
                defer(async { Ok::<_, topcoat_core::error::Error>(["home"]) })
                    .once()
                    .as_key("navigation")
                    .until(Duration::from_mins(1)),
            )],
            &request,
        )
        .await?;

        assert!(!page.props.contains_key("nav"));
        assert!(page.deferred_props.is_empty());
        assert_eq!(page.once_props["navigation"].prop, "nav");
        assert!(page.once_props["navigation"].expires_at.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn explicitly_requested_or_fresh_once_props_are_sent() -> Result<()> {
        let partial = request(&[
            (&header::X_INERTIA, "true"),
            (&header::X_INERTIA_PARTIAL_COMPONENT, "Users/Index"),
            (&header::X_INERTIA_PARTIAL_DATA, "nav"),
            (&header::X_INERTIA_EXCEPT_ONCE_PROPS, "nav"),
        ]);
        let partial_page = page(
            vec![entry(
                "nav",
                lazy(async { Ok::<_, topcoat_core::error::Error>(["home"]) }).once(),
            )],
            &partial,
        )
        .await?;
        assert_eq!(partial_page.props["nav"], json!(["home"]));

        let cached = request(&[
            (&header::X_INERTIA, "true"),
            (&header::X_INERTIA_EXCEPT_ONCE_PROPS, "nav"),
        ]);
        let fresh_page = page(
            vec![entry(
                "nav",
                lazy(async { Ok::<_, topcoat_core::error::Error>(["home"]) })
                    .once()
                    .fresh(),
            )],
            &cached,
        )
        .await?;
        assert_eq!(fresh_page.props["nav"], json!(["home"]));
        Ok(())
    }

    #[tokio::test]
    async fn nested_paths_support_objects_arrays_and_parent_replacement() -> Result<()> {
        let page = page(
            vec![
                entry("users.0.name", value("Ada")),
                entry("users.1.name", value("Grace")),
                entry("settings.theme", value("dark")),
                entry("settings", value(json!({"theme": "light"}))),
            ],
            &InertiaRequest::default(),
        )
        .await?;

        assert_eq!(
            page.props["users"],
            json!([{"name": "Ada"}, {"name": "Grace"}])
        );
        assert_eq!(page.props["settings"], json!({"theme": "light"}));
        Ok(())
    }

    #[tokio::test]
    async fn incompatible_nested_paths_return_an_error() {
        let error = page(
            vec![
                entry("user", value("Ada")),
                entry("user.name", value("Grace")),
            ],
            &InertiaRequest::default(),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("collides"));
    }

    #[tokio::test]
    async fn scroll_metadata_uses_intent_wrapper_and_reset() -> Result<()> {
        let prepend = request(&[
            (&header::X_INERTIA_INFINITE_SCROLL_MERGE_INTENT, "prepend"),
            (&header::X_INERTIA_RESET, "feed"),
        ]);
        let reset_page = page(
            vec![entry(
                "feed",
                scroll(
                    json!({"data": [1]}),
                    ScrollMetadata::new("page")
                        .current_page(2)
                        .previous_page(Some(1))
                        .next_page(Some(3)),
                )
                .wrapper("data"),
            )],
            &prepend,
        )
        .await?;
        assert!(reset_page.prepend_props.is_empty());
        assert!(reset_page.scroll_props["feed"].reset());

        let page = page(
            vec![entry(
                "feed",
                scroll(json!({"data": [1]}), ScrollMetadata::new("page")).wrapper("data"),
            )],
            &request(&[(&header::X_INERTIA_INFINITE_SCROLL_MERGE_INTENT, "prepend")]),
        )
        .await?;
        assert_eq!(page.prepend_props, ["feed.data"]);
        Ok(())
    }
}

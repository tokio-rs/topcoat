use std::collections::BTreeMap;

use serde::Deserialize;

/// The directory under `OUT_DIR` that staged icon sets are written to.
#[doc(hidden)]
pub const STAGE_DIR: &str = "topcoat-icon-iconify";

/// An Iconify icon set in the [`IconifyJSON`] format.
///
/// This is the format Iconify publishes through its `@iconify-json/*`
/// packages, and the format icon sets are staged in for the `include!` and
/// `iconify_icon!` macros. Only the icon data is modeled; metadata such as
/// `info` or `categories` is ignored.
///
/// [`IconifyJSON`]: https://iconify.design/docs/types/iconify-json.html
#[derive(Debug, Clone, Deserialize)]
pub struct IconSet {
    /// The prefix the set's icons are addressed by, e.g. `mdi`.
    pub prefix: String,
    /// The set's icons by name.
    pub icons: BTreeMap<String, Icon>,
    /// The set's aliases by name, each pointing at a parent icon or alias.
    #[serde(default)]
    pub aliases: BTreeMap<String, Alias>,
    /// Default properties for icons that do not set their own.
    #[serde(flatten)]
    pub defaults: Properties,
}

impl IconSet {
    /// Looks up `name` as an icon or alias, following alias parent chains,
    /// applying alias properties, and filling in the set's defaults.
    ///
    /// Returns `None` when the name is unknown or its alias chain does not
    /// terminate in an icon of this set.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<ResolvedIcon<'_>> {
        // Collect the alias chain from `name` down to the icon it ends in.
        let mut chain = Vec::new();
        let mut current = name;
        let icon = loop {
            if let Some(icon) = self.icons.get(current) {
                break icon;
            }
            // A chain longer than the alias map can only be a cycle.
            if chain.len() > self.aliases.len() {
                return None;
            }
            let alias = self.aliases.get(current)?;
            chain.push(alias);
            current = &alias.parent;
        };

        let defaults = &self.defaults;
        let mut resolved = ResolvedIcon {
            body: &icon.body,
            left: icon.properties.left.or(defaults.left).unwrap_or(0.0),
            top: icon.properties.top.or(defaults.top).unwrap_or(0.0),
            width: icon.properties.width.or(defaults.width).unwrap_or(16.0),
            height: icon.properties.height.or(defaults.height).unwrap_or(16.0),
            rotate: icon.properties.rotate.or(defaults.rotate).unwrap_or(0) % 4,
            h_flip: icon.properties.h_flip.or(defaults.h_flip).unwrap_or(false),
            v_flip: icon.properties.v_flip.or(defaults.v_flip).unwrap_or(false),
            hidden: chain.first().map_or(icon.hidden, |alias| alias.hidden),
        };

        // Walk the chain outward from the icon: dimensions override, while
        // transformations merge (rotations add up, flips cancel out).
        for alias in chain.iter().rev() {
            let properties = &alias.properties;
            if let Some(left) = properties.left {
                resolved.left = left;
            }
            if let Some(top) = properties.top {
                resolved.top = top;
            }
            if let Some(width) = properties.width {
                resolved.width = width;
            }
            if let Some(height) = properties.height {
                resolved.height = height;
            }
            resolved.rotate = (resolved.rotate + properties.rotate.unwrap_or(0) % 4) % 4;
            resolved.h_flip ^= properties.h_flip.unwrap_or(false);
            resolved.v_flip ^= properties.v_flip.unwrap_or(false);
        }

        Some(resolved)
    }
}

/// A single icon in an [`IconSet`]: `IconifyJSON`'s `ExtendedIconifyIcon`.
#[derive(Debug, Clone, Deserialize)]
pub struct Icon {
    /// The icon's inner SVG markup, without the `<svg>` element itself.
    pub body: String,
    /// The properties the icon sets explicitly; the set's defaults fill in
    /// the rest.
    #[serde(flatten)]
    pub properties: Properties,
    /// Whether the set hides the icon from icon listings, usually because it
    /// is deprecated. `include!` globs skip hidden entries; addressing one by
    /// name still works.
    #[serde(default)]
    pub hidden: bool,
}

/// An alternate name for an icon in an [`IconSet`]: `IconifyJSON`'s
/// `ExtendedIconifyAlias`.
#[derive(Debug, Clone, Deserialize)]
pub struct Alias {
    /// The name of the icon or alias this alias points at.
    pub parent: String,
    /// The properties the alias overrides. Dimensions replace the parent's,
    /// while transformations merge with them.
    #[serde(flatten)]
    pub properties: Properties,
    /// Whether the set hides the alias from icon listings, usually because
    /// it is deprecated. `include!` globs skip hidden entries; addressing one
    /// by name still works.
    #[serde(default)]
    pub hidden: bool,
}

/// The view box dimensions and transformations of an icon or alias:
/// `IconifyJSON`'s `IconifyOptional`. At the root of a set they act as
/// defaults for icons that do not set their own.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Properties {
    /// The `x` coordinate of the view box origin. Defaults to `0`.
    pub left: Option<f32>,
    /// The `y` coordinate of the view box origin. Defaults to `0`.
    pub top: Option<f32>,
    /// The view box width. Defaults to `16`.
    pub width: Option<f32>,
    /// The view box height. Defaults to `16`.
    pub height: Option<f32>,
    /// The number of 90 degree clockwise rotations. Defaults to `0`.
    pub rotate: Option<u8>,
    /// Whether the icon is flipped horizontally. Defaults to `false`.
    #[serde(rename = "hFlip")]
    pub h_flip: Option<bool>,
    /// Whether the icon is flipped vertically. Defaults to `false`.
    #[serde(rename = "vFlip")]
    pub v_flip: Option<bool>,
}

/// The result of [`IconSet::resolve`]: an icon with aliases followed,
/// properties applied, and defaults filled in.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedIcon<'a> {
    /// The icon's inner SVG markup, without the `<svg>` element itself.
    pub body: &'a str,
    /// The `x` coordinate of the view box origin.
    pub left: f32,
    /// The `y` coordinate of the view box origin.
    pub top: f32,
    /// The view box width.
    pub width: f32,
    /// The view box height.
    pub height: f32,
    /// The number of 90 degree clockwise rotations left to apply, `0..=3`.
    pub rotate: u8,
    /// Whether the icon is still to be flipped horizontally.
    pub h_flip: bool,
    /// Whether the icon is still to be flipped vertically.
    pub v_flip: bool,
    /// Whether the resolved name is hidden: the alias's own flag when a name
    /// resolves through one, the icon's otherwise.
    pub hidden: bool,
}

impl ResolvedIcon<'_> {
    /// Whether the icon carries no net transformation.
    #[must_use]
    pub fn is_untransformed(&self) -> bool {
        self.rotate == 0 && !self.h_flip && !self.v_flip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(json: serde_json::Value) -> IconSet {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn geometry_falls_back_to_set_defaults() {
        let set = set(serde_json::json!({
            "prefix": "demo",
            "icons": {
                "plain": { "body": "<g/>" },
                "sized": { "body": "<g/>", "left": -1, "top": 0.5, "width": 20, "height": 10 },
            },
            "width": 24,
            "height": 24,
        }));

        let plain = set.resolve("plain").unwrap();
        assert_eq!(
            (plain.left, plain.top, plain.width, plain.height),
            (0.0, 0.0, 24.0, 24.0)
        );

        let sized = set.resolve("sized").unwrap();
        assert_eq!(
            (sized.left, sized.top, sized.width, sized.height),
            (-1.0, 0.5, 20.0, 10.0)
        );
    }

    #[test]
    fn geometry_falls_back_to_iconify_defaults() {
        let set = set(serde_json::json!({
            "prefix": "demo",
            "icons": { "plain": { "body": "<g/>" } },
        }));

        let plain = set.resolve("plain").unwrap();
        assert_eq!(
            (plain.left, plain.top, plain.width, plain.height),
            (0.0, 0.0, 16.0, 16.0)
        );
    }

    #[test]
    fn aliases_resolve_through_chains() {
        let set = set(serde_json::json!({
            "prefix": "demo",
            "icons": { "trash": { "body": "<g/>" } },
            "aliases": {
                "bin": { "parent": "trash" },
                "wastebasket": { "parent": "bin" },
            },
        }));

        assert_eq!(set.resolve("wastebasket").unwrap().body, "<g/>");
    }

    #[test]
    fn unknown_names_and_dangling_or_cyclic_aliases_resolve_to_none() {
        let set = set(serde_json::json!({
            "prefix": "demo",
            "icons": { "trash": { "body": "<g/>" } },
            "aliases": {
                "dangling": { "parent": "missing" },
                "ouroboros": { "parent": "ouroboros" },
            },
        }));

        assert!(set.resolve("missing").is_none());
        assert!(set.resolve("dangling").is_none());
        assert!(set.resolve("ouroboros").is_none());
    }

    #[test]
    fn alias_dimensions_override_and_transformations_merge() {
        let set = set(serde_json::json!({
            "prefix": "demo",
            "icons": {
                "arrow": { "body": "<g/>", "width": 24, "height": 24, "rotate": 1 },
            },
            "aliases": {
                "arrow-tall": { "parent": "arrow", "height": 48 },
                "arrow-back": { "parent": "arrow-tall", "rotate": 3, "hFlip": true },
                "arrow-again": { "parent": "arrow-back", "hFlip": true },
            },
        }));

        let tall = set.resolve("arrow-tall").unwrap();
        assert_eq!((tall.width, tall.height), (24.0, 48.0));
        assert_eq!(tall.rotate, 1);

        // 1 + 3 rotations cancel out; the two flips cancel each other too.
        let again = set.resolve("arrow-again").unwrap();
        assert_eq!(again.rotate, 0);
        assert!(!again.h_flip);
        assert!(again.is_untransformed());

        let back = set.resolve("arrow-back").unwrap();
        assert!(back.h_flip);
        assert!(!back.is_untransformed());
    }

    #[test]
    fn hidden_follows_the_named_entry() {
        let set = set(serde_json::json!({
            "prefix": "demo",
            "icons": { "old": { "body": "<g/>", "hidden": true } },
            "aliases": {
                "new": { "parent": "old" },
                "older": { "parent": "old", "hidden": true },
            },
        }));

        assert!(set.resolve("old").unwrap().hidden);
        assert!(!set.resolve("new").unwrap().hidden);
        assert!(set.resolve("older").unwrap().hidden);
    }
}

use crate::{PathBuf, Router, Segment};

#[cfg(feature = "discover")]
#[macro_export]
macro_rules! file_router {
    () => {
        ::topcoat::router::Router::new()
            .file_root(file!())
            .discover()
    };
}

impl Router {
    /// Derives an HTTP route path from a Rust source file path.
    ///
    /// # Examples
    ///
    /// - `./src/app/home.rs` → `/home`
    /// - `./src/app/dashboard/_group/settings/mod.rs` → `/dashboard/settings`
    pub(crate) fn path_from_file(&self, file: &str) -> PathBuf {
        let file_root = self
            .file_root
            .as_deref()
            .expect("determining path from file needs file root");
        let file_root = canonical_module_path(file_root);

        let file = file
            .strip_prefix(file_root)
            .expect("file must be under file router's file root");
        let file = canonical_module_path(file);

        file.split(&['\\', '/'])
            .skip(1)
            .map(|s| {
                if s.starts_with("_") {
                    Segment::Group(s)
                } else {
                    Segment::Static(s)
                }
            })
            .collect()
    }
}

fn canonical_module_path(path: &str) -> &str {
    let path = path.strip_suffix(".rs").unwrap_or(path);
    let path = path.strip_suffix("/mod").unwrap_or(path);
    path.strip_suffix("\\mod").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use crate::Router;

    fn router_with_root(root: &str) -> Router {
        Router::new().file_root(root.to_owned())
    }

    #[test]
    fn simple_file() {
        let router = router_with_root("./src/app.rs");
        assert_eq!(router.path_from_file("./src/app/home.rs").as_str(), "/home");
    }

    #[test]
    fn nested_file() {
        let router = router_with_root("./src/app.rs");
        assert_eq!(
            router
                .path_from_file("./src/app/settings/profile.rs")
                .as_str(),
            "/settings/profile"
        );
    }

    #[test]
    fn mod_rs_becomes_directory() {
        let router = router_with_root("./src/app.rs");
        assert_eq!(
            router.path_from_file("./src/app/settings/mod.rs").as_str(),
            "/settings"
        );
    }

    #[test]
    fn mod_rs_file_root() {
        let router = router_with_root("./src/app/mod.rs");
        assert_eq!(router.path_from_file("./src/app/home.rs").as_str(), "/home");
    }

    #[test]
    fn mod_rs_file_root_with_mod_rs_file() {
        let router = router_with_root("./src/app/mod.rs");
        assert_eq!(
            router.path_from_file("./src/app/settings/mod.rs").as_str(),
            "/settings"
        );
    }
}

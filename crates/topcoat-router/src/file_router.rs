use std::path::Path;

use crate::Router;

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
    /// - `./src/app/settings/mod.rs` → `/settings`
    pub(crate) fn path_from_file(&self, file: &str) -> String {
        let file_root = self
            .file_root
            .as_deref()
            .expect("determining path from file needs file root");

        // Determine root prefix from file_root:
        //   ./src/app.rs     → ./src/app
        //   ./src/app/mod.rs → ./src/app
        let file_root = Path::new(file_root);
        let root_prefix = if file_root.file_name().is_some_and(|name| name == "mod.rs") {
            file_root.parent().unwrap().to_path_buf()
        } else {
            file_root.with_extension("")
        };

        let relative = Path::new(file)
            .strip_prefix(&root_prefix)
            .expect("file must be under file router's file root");

        let without_ext = relative.with_extension("");

        // app/mod → app (mod.rs represents its parent directory)
        let path = if without_ext.file_name().is_some_and(|name| name == "mod") {
            without_ext.parent().unwrap().to_path_buf()
        } else {
            without_ext
        };

        format!("/{}", path.display())
    }
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
        assert_eq!(router.path_from_file("./src/app/home.rs"), "/home");
    }

    #[test]
    fn nested_file() {
        let router = router_with_root("./src/app.rs");
        assert_eq!(
            router.path_from_file("./src/app/settings/profile.rs"),
            "/settings/profile"
        );
    }

    #[test]
    fn mod_rs_becomes_directory() {
        let router = router_with_root("./src/app.rs");
        assert_eq!(
            router.path_from_file("./src/app/settings/mod.rs"),
            "/settings"
        );
    }

    #[test]
    fn mod_rs_file_root() {
        let router = router_with_root("./src/app/mod.rs");
        assert_eq!(router.path_from_file("./src/app/home.rs"), "/home");
    }

    #[test]
    fn mod_rs_file_root_with_mod_rs_file() {
        let router = router_with_root("./src/app/mod.rs");
        assert_eq!(
            router.path_from_file("./src/app/settings/mod.rs"),
            "/settings"
        );
    }
}

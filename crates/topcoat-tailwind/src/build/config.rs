use std::{env, fs, path::PathBuf};

use crate::build::{BuildError, Command, ExecutableSource, Result};

/// The filename of the default [`output`](BuildConfig::output) inside
/// `OUT_DIR`.
pub const DEFAULT_OUTPUT_NAME: &str = "tailwind.css";
const DEFAULT_INPUT_CSS: &str = "@import \"tailwindcss\";\n";

/// Runs the Tailwind CLI from a Cargo build script.
///
/// Create a config with [`BuildConfig::new`], change settings with the
/// builder methods, and run it with [`render`](Self::render). The default
/// config downloads the standalone Tailwind CLI, scans the package for class
/// names, and writes the generated stylesheet to `$OUT_DIR/tailwind.css`:
///
/// ```rust,no_run
/// topcoat::tailwind::BuildConfig::new().render().unwrap();
/// ```
pub struct BuildConfig {
    executable_source: ExecutableSource,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    cwd: Option<PathBuf>,
    optimize: bool,
    minify: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            executable_source: ExecutableSource::default(),
            input: None,
            output: None,
            cwd: None,
            optimize: false,
            minify: true,
        }
    }
}

impl BuildConfig {
    /// Creates a config with every setting at its default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets where the Tailwind CLI executable comes from. Defaults to
    /// downloading [`DEFAULT_VERSION`](crate::DEFAULT_VERSION) from GitHub.
    ///
    /// [`version`](Self::version), [`version_checksum`](Self::version_checksum),
    /// [`executable`](Self::executable), and
    /// [`executable_env`](Self::executable_env) are shorthands for the
    /// variants of [`ExecutableSource`]. If you call several of them, the last
    /// call wins.
    #[must_use]
    pub fn executable_source(mut self, executable_source: ExecutableSource) -> Self {
        self.executable_source = executable_source;
        self
    }

    /// Sets the Tailwind CLI release to download, without the leading `v`,
    /// for example `"4.3.2"`.
    ///
    /// Shorthand for [`ExecutableSource::Github`] without a checksum.
    #[must_use]
    pub fn version(self, version: impl Into<String>) -> Self {
        self.executable_source(ExecutableSource::Github {
            version: version.into(),
            checksum: None,
        })
    }

    /// Sets the Tailwind CLI release to download, together with the expected
    /// hash of the downloaded executable.
    ///
    /// The version has no leading `v`. The checksum has the form
    /// `algorithm:hex`, and the only supported algorithm is `sha256`, for
    /// example `"sha256:b800b065..."`.
    ///
    /// Shorthand for [`ExecutableSource::Github`] with a checksum.
    #[must_use]
    pub fn version_checksum(self, version: impl Into<String>, checksum: impl Into<String>) -> Self {
        self.executable_source(ExecutableSource::Github {
            version: version.into(),
            checksum: Some(checksum.into()),
        })
    }

    /// Uses an installed Tailwind CLI instead of downloading one.
    ///
    /// A plain command name like `"tailwindcss"` is looked up in `PATH`. A
    /// value that contains a path separator is a file path. Relative paths
    /// are relative to the package root, where the build script runs.
    ///
    /// Shorthand for [`ExecutableSource::Path`].
    #[must_use]
    pub fn executable(self, path: impl Into<PathBuf>) -> Self {
        self.executable_source(ExecutableSource::Path(path.into()))
    }

    /// Reads the path of the Tailwind CLI from the environment variable `name`
    /// when the build script runs.
    ///
    /// The value is interpreted like the argument of
    /// [`executable`](Self::executable). If a change to the variable should
    /// rerun the build script, print `cargo:rerun-if-env-changed=<name>`
    /// yourself.
    ///
    /// Shorthand for [`ExecutableSource::Env`].
    #[must_use]
    pub fn executable_env(self, name: impl Into<String>) -> Self {
        self.executable_source(ExecutableSource::Env(name.into()))
    }

    /// Sets the input CSS file.
    ///
    /// Defaults to a generated `tailwind-input.css` in `OUT_DIR` that only
    /// contains `@import "tailwindcss";`. The Tailwind CLI resolves a relative
    /// path against [`cwd`](Self::cwd), which defaults to the package root.
    #[must_use]
    pub fn input(mut self, path: impl Into<PathBuf>) -> Self {
        self.input = Some(path.into());
        self
    }

    /// Sets the output CSS file.
    ///
    /// Defaults to `$OUT_DIR/tailwind.css`, the file that
    /// [`stylesheet!`](crate::stylesheet) declares as an asset. The Tailwind
    /// CLI resolves a relative path against [`cwd`](Self::cwd), which defaults
    /// to the package root.
    #[must_use]
    pub fn output(mut self, path: impl Into<PathBuf>) -> Self {
        self.output = Some(path.into());
        self
    }

    /// Sets the directory passed to the Tailwind CLI with `--cwd`.
    ///
    /// Tailwind scans this directory for class names, and resolves relative
    /// [`input`](Self::input) and [`output`](Self::output) paths against it.
    /// Defaults to `$CARGO_MANIFEST_DIR`, the package root.
    ///
    /// Tailwind scans every file in this directory that `.gitignore` does not
    /// exclude. The `.gitignore` that Cargo generates excludes `target/`.
    /// Without it, Tailwind also scans build output, which is slow and can
    /// bring back class names from earlier builds. If you cannot rely on a
    /// `.gitignore`, scan a smaller directory, for example `.cwd("src")`. Or
    /// turn off directory scanning with a custom [`input`](Self::input) that
    /// uses `@import "tailwindcss" source(none)` and explicit `@source` globs.
    #[must_use]
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Sets whether to pass `--optimize` to the Tailwind CLI. Defaults to
    /// `false`.
    #[must_use]
    pub fn optimize(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }

    /// Sets whether to pass `--minify` to the Tailwind CLI. Defaults to
    /// `true`.
    #[must_use]
    pub fn minify(mut self, minify: bool) -> Self {
        self.minify = minify;
        self
    }

    /// Gets the Tailwind CLI from the configured [`ExecutableSource`] and runs
    /// it. Returns the path of the generated CSS file.
    ///
    /// Call this from a build script. It needs `OUT_DIR` when the CLI is
    /// downloaded or when [`input`](Self::input) or [`output`](Self::output)
    /// is not set, and `CARGO_MANIFEST_DIR` when [`cwd`](Self::cwd) is not
    /// set. Cargo sets both for build scripts.
    ///
    /// # Errors
    ///
    /// Returns an error if the CLI cannot be downloaded, does not match its
    /// checksum, or cannot be run, if the variable of an
    /// [`ExecutableSource::Env`] is not set, if the Tailwind CLI exits with a
    /// non-zero status, or if `OUT_DIR` or `CARGO_MANIFEST_DIR` is needed but
    /// not set.
    pub fn render(self) -> Result<PathBuf> {
        let executable = self.executable_source.resolve()?;

        let out_dir = env::var_os("OUT_DIR").map(PathBuf::from);

        let input = if let Some(path) = self.input {
            path
        } else {
            let out_dir = out_dir.as_deref().ok_or(BuildError::NoOutDir)?;
            let path = out_dir.join("tailwind-input.css");
            // Only write if contents would change: the file's mtime then
            // stays stable, so a user-supplied `rerun-if-changed` directive
            // for it doesn't rerun the build script every build.
            let needs_write = match fs::read(&path) {
                Ok(existing) => existing != DEFAULT_INPUT_CSS.as_bytes(),
                Err(_) => true,
            };
            if needs_write {
                fs::write(&path, DEFAULT_INPUT_CSS).map_err(|source| BuildError::Io {
                    path: path.clone(),
                    source,
                })?;
            }
            path
        };

        let output = if let Some(path) = self.output {
            path
        } else {
            out_dir
                .as_deref()
                .ok_or(BuildError::NoOutDir)?
                .join(DEFAULT_OUTPUT_NAME)
        };

        let cwd = if let Some(path) = self.cwd {
            path
        } else {
            let manifest_dir =
                env::var_os("CARGO_MANIFEST_DIR").ok_or(BuildError::NoManifestDir)?;
            PathBuf::from(manifest_dir)
        };

        let command = Command {
            input,
            output: output.clone(),
            cwd,
            optimize: self.optimize,
            minify: self.minify,
        };
        executable.run(&command)?;

        Ok(output)
    }
}

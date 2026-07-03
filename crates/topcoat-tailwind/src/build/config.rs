use std::{env, fs, path::PathBuf};

use crate::build::{BuildError, Command, ExecutableSource, Result};

pub const DEFAULT_OUTPUT_NAME: &str = "tailwind.css";
const DEFAULT_INPUT_CSS: &str = "@import \"tailwindcss\";\n";

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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Where the Tailwind CLI executable comes from. Defaults to downloading
    /// [`DEFAULT_VERSION`](crate::build::DEFAULT_VERSION) from GitHub.
    ///
    /// [`version`](Self::version), [`version_checksum`](Self::version_checksum),
    /// [`executable`](Self::executable), and
    /// [`executable_env`](Self::executable_env) are shorthands for the
    /// individual variants; the most recent call wins.
    #[must_use]
    pub fn executable_source(mut self, executable_source: ExecutableSource) -> Self {
        self.executable_source = executable_source;
        self
    }

    /// Pin the Tailwind CLI release to download (without the leading `v`).
    ///
    /// Shorthand for [`ExecutableSource::Github`] without a checksum.
    #[must_use]
    pub fn version(self, version: impl Into<String>) -> Self {
        self.executable_source(ExecutableSource::Github {
            version: version.into(),
            checksum: None,
        })
    }

    /// Pin the Tailwind CLI release to download (without the leading `v`)
    /// along with the expected SHA-256 of the downloaded binary, as lowercase
    /// hex.
    ///
    /// Shorthand for [`ExecutableSource::Github`] with a checksum.
    #[must_use]
    pub fn version_checksum(
        self,
        version: impl Into<String>,
        checksum: impl Into<String>,
    ) -> Self {
        self.executable_source(ExecutableSource::Github {
            version: version.into(),
            checksum: Some(checksum.into()),
        })
    }

    /// Use an existing Tailwind CLI executable instead of downloading one.
    ///
    /// A bare command name like `"tailwindcss"` is resolved through `PATH`;
    /// anything containing a path separator is used as a file path, with
    /// relative paths resolved against the package root (the directory the
    /// build script runs in).
    ///
    /// Shorthand for [`ExecutableSource::Path`].
    #[must_use]
    pub fn executable(self, path: impl Into<PathBuf>) -> Self {
        self.executable_source(ExecutableSource::Path(path.into()))
    }

    /// Read the Tailwind CLI executable from an environment variable at build
    /// time. The variable's value is interpreted like
    /// [`executable`](Self::executable), and the build script reruns when the
    /// variable changes.
    ///
    /// Shorthand for [`ExecutableSource::Env`].
    #[must_use]
    pub fn executable_env(self, name: impl Into<String>) -> Self {
        self.executable_source(ExecutableSource::Env(name.into()))
    }

    /// Input CSS file. Defaults to a generated `input.css` in `OUT_DIR` that
    /// just contains `@import "tailwindcss";`.
    #[must_use]
    pub fn input(mut self, path: impl Into<PathBuf>) -> Self {
        self.input = Some(path.into());
        self
    }

    /// Output CSS file. Defaults to `$OUT_DIR/tailwind.css`, which can be
    /// loaded from source via `asset!(concat!(env!("OUT_DIR"), "/tailwind.css"))`.
    #[must_use]
    pub fn output(mut self, path: impl Into<PathBuf>) -> Self {
        self.output = Some(path.into());
        self
    }

    /// Pass `--cwd` to the Tailwind CLI. Defaults to `$CARGO_MANIFEST_DIR/src`.
    #[must_use]
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Pass `--optimize` to the Tailwind CLI. Defaults to `false`.
    #[must_use]
    pub fn optimize(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }

    /// Pass `--minify` to the Tailwind CLI. Defaults to `true`.
    #[must_use]
    pub fn minify(mut self, minify: bool) -> Self {
        self.minify = minify;
        self
    }

    /// Resolve the Tailwind CLI executable from the configured
    /// [`ExecutableSource`] and run it. Returns the path to the generated CSS
    /// file.
    ///
    /// `OUT_DIR` is only required when something depends on it: the CLI is
    /// downloaded, or `input`/`output` is left at its default.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the CLI cannot be downloaded, fails checksum
    /// verification, or cannot be executed, if an [`ExecutableSource::Env`]
    /// variable is unset, if the Tailwind CLI exits with a non-zero status, or
    /// if `OUT_DIR` or `CARGO_MANIFEST_DIR` is unset while a default depends
    /// on it.
    pub fn render(self) -> Result<PathBuf> {
        let executable = self.executable_source.resolve()?;

        let out_dir = env::var_os("OUT_DIR").map(PathBuf::from);

        let input = if let Some(path) = self.input {
            path
        } else {
            let out_dir = out_dir.as_deref().ok_or(BuildError::NoOutDir)?;
            let path = out_dir.join("tailwind-input.css");
            // Only write if contents would change; otherwise the file's
            // mtime advances every build and the `rerun-if-changed` below
            // forces the build script to run again next time.
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
            PathBuf::from(manifest_dir).join("src")
        };

        println!("cargo:rerun-if-changed={}", input.display());
        println!("cargo:rerun-if-changed={}", cwd.display());

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

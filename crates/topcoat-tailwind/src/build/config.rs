use std::{env, fs, path::PathBuf, process::Command};

use crate::build::{BuildError, Result, executable};

pub const DEFAULT_VERSION: &str = "4.3.0";
pub const DEFAULT_OUTPUT_NAME: &str = "tailwind.css";
const DEFAULT_INPUT_CSS: &str = "@import \"tailwindcss\";\n";

pub struct BuildConfig {
    version: String,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    cwd: Option<PathBuf>,
    optimize: bool,
    minify: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            version: DEFAULT_VERSION.to_owned(),
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

    /// Pin the Tailwind CLI release (without the leading `v`).
    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
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

    /// Download the CLI if needed and run it. Returns the path to the
    /// generated CSS file.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `OUT_DIR` or `CARGO_MANIFEST_DIR` is unset, if the CLI
    /// cannot be downloaded or executed, or if the Tailwind CLI exits with a
    /// non-zero status.
    pub fn render(self) -> Result<PathBuf> {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or(BuildError::NoOutDir)?);

        let cli = out_dir.join(format!("tailwindcss-{}", self.version));
        executable::download(&self.version, &cli)?;

        let input = if let Some(path) = self.input {
            path
        } else {
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

        let output = self
            .output
            .unwrap_or_else(|| out_dir.join(DEFAULT_OUTPUT_NAME));

        let cwd = if let Some(path) = self.cwd {
            path
        } else {
            let manifest_dir =
                env::var_os("CARGO_MANIFEST_DIR").ok_or(BuildError::NoManifestDir)?;
            PathBuf::from(manifest_dir).join("src")
        };

        println!("cargo:rerun-if-changed={}", input.display());
        println!("cargo:rerun-if-changed={}", cwd.display());

        let mut command = Command::new(&cli);
        command
            .arg("-i")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .arg("--cwd")
            .arg(&cwd);
        if self.optimize {
            command.arg("--optimize");
        }
        if self.minify {
            command.arg("--minify");
        }
        let status = command.status().map_err(|source| BuildError::Io {
            path: cli.clone(),
            source,
        })?;

        if !status.success() {
            return Err(BuildError::Cli { status });
        }

        Ok(output)
    }
}

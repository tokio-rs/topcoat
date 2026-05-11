use std::{env, fs, path::PathBuf, process::Command};

use crate::build::{BuildError, Result, executable};

pub const DEFAULT_VERSION: &str = "4.3.0";
pub const DEFAULT_OUTPUT_NAME: &str = "tailwind.css";
const DEFAULT_INPUT_CSS: &str = "@import \"tailwindcss\";\n";

pub struct BuildConfig {
    version: String,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            version: DEFAULT_VERSION.to_owned(),
            input: None,
            output: None,
        }
    }
}

impl BuildConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pin the Tailwind CLI release (without the leading `v`).
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Input CSS file. Defaults to a generated `input.css` in `OUT_DIR` that
    /// just contains `@import "tailwindcss";`.
    pub fn input(mut self, path: impl Into<PathBuf>) -> Self {
        self.input = Some(path.into());
        self
    }

    /// Output CSS file. Defaults to `$OUT_DIR/tailwind.css`, which can be
    /// loaded from source via `asset!(concat!(env!("OUT_DIR"), "/tailwind.css"))`.
    pub fn output(mut self, path: impl Into<PathBuf>) -> Self {
        self.output = Some(path.into());
        self
    }

    /// Download the CLI if needed and run it. Returns the path to the
    /// generated CSS file.
    pub fn render(self) -> Result<PathBuf> {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or(BuildError::NoOutDir)?);

        let cli = out_dir.join(format!("tailwindcss-{}", self.version));
        executable::download(&self.version, &cli)?;

        let input = match self.input {
            Some(path) => path,
            None => {
                let path = out_dir.join("tailwind-input.css");
                fs::write(&path, DEFAULT_INPUT_CSS).map_err(|source| BuildError::Io {
                    path: path.clone(),
                    source,
                })?;
                path
            }
        };

        let output = self
            .output
            .unwrap_or_else(|| out_dir.join(DEFAULT_OUTPUT_NAME));

        println!("cargo:rerun-if-changed={}", input.display());

        let status = Command::new(&cli)
            .arg("-i")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .arg("--minify")
            .status()
            .map_err(|source| BuildError::Io {
                path: cli.clone(),
                source,
            })?;

        if !status.success() {
            return Err(BuildError::Cli { status });
        }

        Ok(output)
    }
}

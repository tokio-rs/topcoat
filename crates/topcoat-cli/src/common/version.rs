//! Checks whether the CLI is compatible with a project's Topcoat dependency.
//!
//! Compatibility is based on the CLI version and the project's lockfile.

use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

use console::style;
use serde::Deserialize;

/// The crate whose version the CLI is released in lockstep with.
const FACADE: &str = "topcoat";

/// The file the project's resolved `topcoat` version is read from. It sits at
/// the root of the cargo workspace.
const LOCKFILE: &str = "Cargo.lock";

/// The environment variable that silences the check.
const OPT_OUT: &str = "TOPCOAT_NO_VERSION_CHECK";

/// Warns if the project resolves an incompatible Topcoat version.
///
/// The check never fails a command. It is skipped without a workspace lockfile or
/// Topcoat dependency, or when [`OPT_OUT`] is set.
pub fn warn_on_mismatch() {
    if std::env::var_os(OPT_OUT).is_some() {
        return;
    }

    let Some(cli) = Compat::of(env!("CARGO_PKG_VERSION")) else {
        return;
    };
    let Ok(dir) = std::env::current_dir() else {
        return;
    };
    let Some(lockfile) = lockfile_path(&dir).and_then(|path| std::fs::read_to_string(path).ok())
    else {
        return;
    };

    for mismatch in mismatches(&lockfile, cli) {
        eprintln!(
            "{} this project depends on {FACADE} {}, but the {FACADE} CLI is {}",
            style("warning:").yellow().bold(),
            mismatch.version,
            env!("CARGO_PKG_VERSION"),
        );
        eprintln!(
            "  install a matching CLI with `cargo install topcoat-cli@{} --locked`",
            mismatch.compat,
        );
    }
}

/// The nearest [`LOCKFILE`] at or above `dir`, which is the one cargo resolves
/// the dependencies of a project in `dir` with.
fn lockfile_path(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .map(|dir| dir.join(LOCKFILE))
        .find(|path| path.is_file())
}

/// A resolved Topcoat version incompatible with the CLI.
struct Mismatch {
    /// The version cargo resolved for the project.
    version: String,
    /// The CLI versions that can drive it.
    compat: Compat,
}

/// Finds incompatible Topcoat versions in the lockfile. Reports each incompatible
/// version separately.
fn mismatches(lockfile: &str, cli: Compat) -> Vec<Mismatch> {
    let Ok(lockfile) = toml::from_str::<Lockfile>(lockfile) else {
        return Vec::new();
    };

    lockfile
        .package
        .into_iter()
        .filter(|package| package.name == FACADE)
        .filter_map(|package| {
            let compat = Compat::of(&package.version)?;
            (compat != cli).then_some(Mismatch {
                version: package.version,
                compat,
            })
        })
        .collect()
}

/// The part of a `Cargo.lock` the check reads.
#[derive(Deserialize)]
struct Lockfile {
    #[serde(default)]
    package: Vec<LockedPackage>,
}

/// A dependency resolved in a `Cargo.lock`.
#[derive(Deserialize)]
struct LockedPackage {
    name: String,
    version: String,
}

/// A group of versions treated as compatible by Cargo's semver rules.
///
/// Two versions are compatible when their groups match. [`Display`] formats the group
/// as a version requirement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Compat {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Compat {
    /// The compatibility of a semver version, ignoring its pre-release and
    /// build metadata. Returns `None` when the version does not parse.
    fn of(version: &str) -> Option<Self> {
        let core = version.split(['-', '+']).next()?;
        let mut fields = core.split('.').map(str::parse::<u64>);
        let major = fields.next()?.ok()?;
        let minor = fields.next().transpose().ok()?.unwrap_or_default();
        let patch = fields.next().transpose().ok()?.unwrap_or_default();

        // Leading zeroes mark the unstable part of a version, so each of them
        // shifts compatibility one field to the right.
        Some(match (major, minor) {
            (0, 0) => Self {
                major: 0,
                minor: 0,
                patch,
            },
            (0, minor) => Self {
                major: 0,
                minor,
                patch: 0,
            },
            (major, _) => Self {
                major,
                minor: 0,
                patch: 0,
            },
        })
    }
}

impl Display for Compat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.major, self.minor) {
            (0, 0) => write!(f, "0.0.{}", self.patch),
            (0, minor) => write!(f, "0.{minor}"),
            (major, _) => write!(f, "{major}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lockfile resolving `topcoat` to every one of `versions`, alongside an
    /// unrelated crate.
    fn lockfile(versions: &[&str]) -> String {
        use std::fmt::Write;

        let mut lockfile =
            String::from("version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n");
        for version in versions {
            write!(
                lockfile,
                "\n[[package]]\nname = \"topcoat\"\nversion = \"{version}\"\n"
            )
            .unwrap();
        }
        lockfile
    }

    #[test]
    fn compatible_versions_share_a_compatibility() {
        assert_eq!(Compat::of("1.2.0"), Compat::of("1.5.3"));
        assert_eq!(Compat::of("0.5.0"), Compat::of("0.5.1"));
        assert_eq!(Compat::of("0.0.3"), Compat::of("0.0.3"));
        assert_eq!(Compat::of("0.5.0"), Compat::of("0.5"));
        assert_eq!(Compat::of("0.5.0"), Compat::of("0.5.0-alpha.1"));
        assert_eq!(Compat::of("0.5.0"), Compat::of("0.5.0+build.7"));

        assert_ne!(Compat::of("0.5.0"), Compat::of("0.6.0"));
        assert_ne!(Compat::of("0.5.0"), Compat::of("1.5.0"));
        assert_ne!(Compat::of("0.0.3"), Compat::of("0.0.4"));
        assert_ne!(Compat::of("1.0.0"), Compat::of("2.0.0"));
    }

    #[test]
    fn a_compatibility_renders_as_an_install_requirement() {
        assert_eq!(Compat::of("0.5.1").unwrap().to_string(), "0.5");
        assert_eq!(Compat::of("0.0.3").unwrap().to_string(), "0.0.3");
        assert_eq!(Compat::of("1.5.3").unwrap().to_string(), "1");
    }

    #[test]
    fn a_version_that_does_not_parse_has_no_compatibility() {
        assert_eq!(Compat::of(""), None);
        assert_eq!(Compat::of("nightly"), None);
        assert_eq!(Compat::of("0.x.0"), None);
    }

    #[test]
    fn a_compatible_project_does_not_mismatch() {
        let cli = Compat::of("0.5.0").unwrap();

        assert!(mismatches(&lockfile(&["0.5.0"]), cli).is_empty());
        assert!(mismatches(&lockfile(&["0.5.9"]), cli).is_empty());
    }

    #[test]
    fn a_project_without_topcoat_does_not_mismatch() {
        let cli = Compat::of("0.5.0").unwrap();

        assert!(mismatches(&lockfile(&[]), cli).is_empty());
        assert!(mismatches("", cli).is_empty());
        assert!(mismatches("not a lockfile", cli).is_empty());
    }

    #[test]
    fn an_incompatible_project_reports_the_matching_cli() {
        let cli = Compat::of("0.5.0").unwrap();
        let mismatches = mismatches(&lockfile(&["0.6.1"]), cli);

        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].version, "0.6.1");
        assert_eq!(mismatches[0].compat.to_string(), "0.6");
    }

    #[test]
    fn every_incompatible_version_is_reported() {
        let cli = Compat::of("0.5.0").unwrap();
        let mismatches = mismatches(&lockfile(&["0.4.2", "0.5.0", "1.0.0"]), cli);

        let reported: Vec<&str> = mismatches
            .iter()
            .map(|mismatch| mismatch.version.as_str())
            .collect();
        assert_eq!(reported, ["0.4.2", "1.0.0"]);
    }
}

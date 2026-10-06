//! Compiles starter applications created by `topcoat new` against the framework crates
//! in this repository. Compiling is slow and downloads dependencies, Tailwind, and icon
//! sets, so these tests are ignored by default. Run them with:
//!
//! ```sh
//! cargo test -p topcoat-cli --test starters -- --ignored
//! ```

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::SystemTime,
};

#[test]
#[ignore = "slow; compiles a starter application"]
fn recommended_preset_compiles() {
    check("recommended", &["--recommended"], true);
}

#[test]
#[ignore = "slow; compiles a starter application"]
fn minimal_preset_compiles() {
    check("minimal", &["--minimal"], false);
}

/// The choices neither preset covers: manual routing, htmx, Tailwind without Topcoat
/// UI, custom icons, and a font other than the UI theme's.
#[test]
#[ignore = "slow; compiles a starter application"]
fn mixed_choices_compile() {
    check(
        "mixed",
        &[
            "--routing",
            "manual",
            "--database",
            "none",
            "--interaction",
            "htmx",
            "--tailwind",
            "--icons",
            "custom",
            "--font",
            "fontsource",
            "--font-family",
            "inter",
            "--no-ui",
        ],
        true,
    );
}

/// Generates an application with `args`, compiles it without warnings, and, with
/// `tailwind`, checks that the stylesheet has a rule for every class the pages use.
fn check(name: &str, args: &[&str], tailwind: bool) {
    // Outside the repository, so the application is neither part of this workspace
    // nor covered by its ignore rules.
    let root = std::env::temp_dir().join(format!("topcoat-starters-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let package = format!("starter-{name}");
    let project = root.join(&package);
    let _ = fs::remove_dir_all(&project);

    let output = Command::new(env!("CARGO_BIN_EXE_topcoat"))
        .arg("new")
        .arg(&project)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "topcoat new failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Build against this checkout instead of the published release. The generated
    // manifest itself never contains local paths.
    let framework = Path::new(env!("CARGO_MANIFEST_DIR")).join("../topcoat");
    let manifest = project.join("Cargo.toml");
    let mut contents = fs::read_to_string(&manifest).unwrap();
    // A literal TOML string needs no escaping, even for Windows paths.
    writeln!(
        contents,
        "\n[patch.crates-io]\ntopcoat = {{ path = '{}' }}",
        framework.canonicalize().unwrap().display()
    )
    .unwrap();
    fs::write(&manifest, contents).unwrap();

    // Applications share a target directory, so dependencies with the same features
    // compile once.
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("starters");
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest)
        .env("CARGO_TARGET_DIR", &target)
        .env("RUSTFLAGS", "-Dwarnings")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{package} does not compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    if tailwind {
        let css = fs::read_to_string(stylesheet(&target, &package)).unwrap();
        for class in page_classes(&project.join("src")) {
            assert!(
                has_rule(&css, &class),
                "the stylesheet of {package} has no rule for `{class}`"
            );
        }
    }

    fs::remove_dir_all(&project).unwrap();
}

/// The Tailwind output of the most recent build of `package`.
fn stylesheet(target: &Path, package: &str) -> PathBuf {
    let prefix = format!("{package}-");
    fs::read_dir(target.join("debug/build"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|dir| {
            dir.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .map(|dir| dir.join("out/tailwind.css"))
        .filter_map(|css| {
            let modified = css.metadata().ok()?.modified().ok()?;
            Some((modified, css))
        })
        .max_by_key(|(modified, _): &(SystemTime, PathBuf)| *modified)
        .unwrap_or_else(|| panic!("no Tailwind output for {package}"))
        .1
}

/// The classes in literal `class="..."` attributes of the route tree, `src/app.rs` and
/// `src/app/`.
fn page_classes(src: &Path) -> Vec<String> {
    let mut files = vec![src.join("app.rs")];
    let mut dirs = vec![src.join("app")];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    let mut classes = Vec::new();
    for file in files {
        let source = fs::read_to_string(file).unwrap();
        for attribute in source.split("class=\"").skip(1) {
            let value = attribute.split('"').next().unwrap();
            classes.extend(value.split_whitespace().map(ToString::to_string));
        }
    }
    classes.sort_unstable();
    classes.dedup();
    classes
}

/// Whether `css` has a selector for `class`. Tailwind escapes characters other than
/// letters, digits, `-`, and `_` with a backslash.
fn has_rule(css: &str, class: &str) -> bool {
    let mut selector = String::from(".");
    for char in class.chars() {
        if !(char.is_ascii_alphanumeric() || char == '-' || char == '_') {
            selector.push('\\');
        }
        selector.push(char);
    }
    css.match_indices(&selector).any(|(index, _)| {
        css[index + selector.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_ascii_alphanumeric() || next == '-' || next == '_'))
    })
}

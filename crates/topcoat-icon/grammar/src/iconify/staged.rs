use std::{
    collections::HashMap,
    env,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use proc_macro2::Span;

use topcoat_icon::iconify::{IconSet, STAGE_DIR};

use crate::iconify::suggest::did_you_mean;

/// The staged sets parsed by this process, by staged file path.
static CACHE: OnceLock<Mutex<HashMap<PathBuf, &'static IconSet>>> = OnceLock::new();

/// Loads the staged icon set `prefix` of the crate whose macros are being
/// expanded, located through the `OUT_DIR` its build script staged into.
///
/// Sets are parsed once per process and cached by path, since the proc-macro
/// server expands many invocations in one process.
pub(crate) fn staged_set(prefix: &str, span: Span) -> syn::Result<&'static IconSet> {
    let error = |message| Err(syn::Error::new(span, message));

    let Some(out_dir) = env::var_os("OUT_DIR") else {
        return error(format!(
            "`OUT_DIR` is not set: Iconify icon sets are staged by a build script; add a \
             `build.rs` to this crate:\n\n{hint}",
            hint = stage_hint(prefix),
        ));
    };
    let path = PathBuf::from(out_dir)
        .join(STAGE_DIR)
        .join(format!("{prefix}.json"));

    let cache = CACHE.get_or_init(Mutex::default);
    if let Some(set) = cache.lock().unwrap().get(&path) {
        return Ok(set);
    }

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return error(unknown_set(prefix, path.parent().unwrap_or(&path)));
        }
        Err(source) => {
            return error(format!(
                "failed to read the staged icon set `{prefix}` at `{path}`: {source}",
                path = path.display(),
            ));
        }
    };
    let set: IconSet = match serde_json::from_slice(&bytes) {
        Ok(set) => set,
        Err(source) => {
            return error(format!(
                "the staged icon set `{prefix}` is not valid Iconify JSON ({source}); rerun the \
                 build script that staged it, e.g. by touching `build.rs`",
            ));
        }
    };

    let set = Box::leak(Box::new(set));
    cache.lock().unwrap().insert(path, set);
    Ok(set)
}

/// The message for a prefix that no staged file exists for: near misses among
/// the staged sets, what is staged, and how to stage more.
fn unknown_set(prefix: &str, dir: &Path) -> String {
    let staged = staged_prefixes(dir);
    if staged.is_empty() {
        return format!(
            "no Iconify icon sets are staged; stage `{prefix}` from this crate's \
             `build.rs`:\n\n{hint}",
            hint = stage_hint(prefix),
        );
    }

    let mut message = format!("the Iconify icon set `{prefix}` is not staged");
    if let Some(did_you_mean) = did_you_mean(prefix, staged.iter().map(String::as_str)) {
        message.push_str("; ");
        message.push_str(&did_you_mean);
    }
    message.push_str("\n\nstaged sets: ");
    message.push_str(&staged.join(", "));
    let _ = write!(
        message,
        "\n\nadditional sets are staged from this crate's `build.rs`:\n\n{hint}",
        hint = stage_hint(prefix),
    );
    message
}

/// The prefixes with a staged file in `dir`, sorted.
fn staged_prefixes(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut prefixes: Vec<String> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()? != "json" {
                return None;
            }
            Some(path.file_stem()?.to_str()?.to_owned())
        })
        .collect();
    prefixes.sort();
    prefixes
}

/// A build script snippet staging `set`, for error messages.
fn stage_hint(set: &str) -> String {
    format!(
        "fn main() {{\n    \
             topcoat::icon::iconify::BuildConfig::new()\n        \
                 .icon_set(\"{set}\")\n        \
                 .stage()\n        \
                 .unwrap();\n\
         }}"
    )
}

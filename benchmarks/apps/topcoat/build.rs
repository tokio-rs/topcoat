use std::env;

fn main() {
    let config = topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        // A fixed path instead of `OUT_DIR`, whose name changes with the enabled cargo
        // features. See `STYLESHEET` in `src/app.rs`.
        .output("target/tailwind.css");

    // The benchmark shell's Tailwind CLI, so every app compiles its stylesheet with the same
    // release. Builds outside the shell, like CI, use the pinned download instead.
    println!("cargo::rerun-if-env-changed=PATH");
    let on_path = env::var_os("PATH")
        .is_some_and(|path| env::split_paths(&path).any(|dir| dir.join("tailwindcss").is_file()));
    let config = if on_path {
        config.executable("tailwindcss")
    } else {
        config
    };

    config.render().unwrap();
}

use std::process::Command;

fn main() {
    let rustc = std::env::var_os("RUSTC").expect("Cargo sets RUSTC");
    let version = Command::new(&rustc).arg("-Vv").output().unwrap();
    let version = String::from_utf8(version.stdout).unwrap();
    assert!(
        version.contains("commit-hash: 1ed2df61a19042f231709eb05d032ae9e2cb2084"),
        "topcoat-split requires nightly-2026-08-05 (rustc_driver is version-specific)"
    );
    let sysroot = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    let sysroot = String::from_utf8(sysroot.stdout).unwrap();
    println!("cargo:rustc-env=SPLIT_SYSROOT={}", sysroot.trim());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib", sysroot.trim());
    println!("cargo:rerun-if-changed=build.rs");
}

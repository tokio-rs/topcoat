use std::process::{Command, Stdio};

#[test]
fn help_and_version_accept_cargo_and_direct_invocations() {
    for (binary, prefix) in [
        (env!("CARGO_BIN_EXE_topcoat"), &[][..]),
        (env!("CARGO_BIN_EXE_cargo-topcoat"), &[][..]),
        (env!("CARGO_BIN_EXE_cargo-topcoat"), &["topcoat"][..]),
    ] {
        for args in [&["--help"][..], &["--version"][..], &["fmt", "--help"][..]] {
            let output = Command::new(binary)
                .args(prefix)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{binary} {prefix:?} {args:?}: {}",
                String::from_utf8_lossy(&output.stderr),
            );
            assert!(!output.stdout.is_empty());
        }
    }
}

#[test]
fn cargo_invocation_runs_a_subcommand() {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-topcoat"))
        .args(["topcoat", "fmt", "--stdin"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn cargo_invocation_preserves_invalid_arguments() {
    for args in [
        &["topcoat", "unknown"][..],
        &["topcoat", "topcoat", "--help"][..],
        &["topcoat", "fmt", "--unknown"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_cargo-topcoat"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{args:?}");
    }
}

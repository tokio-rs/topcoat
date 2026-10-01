use std::{
    path::PathBuf,
    process::{Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use tokio::{io::AsyncWriteExt, process::Command};

struct Project {
    path: PathBuf,
}

impl Project {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "topcoat-fmt-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("rustfmt.toml"), "edition = \"2024\"\n").unwrap();
        Self { path }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_topcoat"));
        command
            .arg("fmt")
            .args(args)
            .current_dir(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.path).unwrap();
    }
}

async fn run(mut command: Command, input: &str) -> Output {
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut child = command.spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let (written, output) = tokio::join!(
            async move { stdin.write_all(input.as_bytes()).await },
            child.wait_with_output(),
        );
        let output = output.unwrap();
        if output.status.success() {
            written.unwrap();
        }
        output
    })
    .await
    .expect("formatter timed out")
}

fn formatted(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
async fn stdin_formats_rust_then_macros_and_is_idempotent() {
    let project = Project::new();
    let input =
        "async fn example(){let x=1;view!{<div id = \"greeting\"><p>\"hello\"</p></div>}}\n";
    let output = formatted(run(project.command(&["--stdin", "--rustfmt"]), input).await);
    assert!(output.starts_with("async fn example() {\n    let x = 1;\n"));
    assert!(output.contains("<div id=\"greeting\">"));
    assert!(output.contains("<p>\"hello\"</p>"));
    assert_eq!(
        formatted(run(project.command(&["--stdin", "--rustfmt"]), &output).await),
        output,
    );
}

#[tokio::test]
async fn macro_only_mode_does_not_require_rustfmt_or_change_surrounding_rust() {
    let project = Project::new();
    let mut command = project.command(&["--stdin"]);
    command.env("PATH", "");
    let output = formatted(
        run(
            command,
            "fn example(){view!{<div id = \"greeting\"><p>\"hello\"</p></div>}}",
        )
        .await,
    );
    assert!(output.starts_with("fn example(){"));
    assert!(output.contains("<div id=\"greeting\">"));
}

#[tokio::test]
async fn macro_selection_is_preserved_with_rustfmt() {
    let project = Project::new();
    let output = formatted(
        run(
            project.command(&["--stdin", "--rustfmt", "--macros", "class"]),
            "fn example(){view!{<div id = \"greeting\"><p>\"hello\"</p></div>}}",
        )
        .await,
    );
    assert!(output.starts_with("fn example() {\n"));
    assert!(output.contains("<div id = \"greeting\"><p>\"hello\"</p></div>"));
}

#[tokio::test]
async fn files_use_nearest_rustfmt_config_without_following_modules() {
    let project = Project::new();
    std::fs::create_dir(project.path.join("src")).unwrap();
    std::fs::write(
        project.path.join("src/rustfmt.toml"),
        "edition = \"2024\"\ntab_spaces = 2\n",
    )
    .unwrap();
    let path = project.path.join("src/main.rs");
    std::fs::write(&path, "mod missing;\nasync fn main(){let x=1;}\n").unwrap();
    let output = run(project.command(&["--rustfmt", "src/main.rs"]), "").await;
    assert_eq!(formatted(output), "");
    let source = std::fs::read_to_string(path).unwrap();
    assert!(source.contains("async fn main() {\n  let x = 1;\n}"));
}

#[tokio::test]
async fn default_file_discovery_runs_both_formatters() {
    let project = Project::new();
    let path = project.path.join("main.rs");
    std::fs::write(
        &path,
        "fn main(){view!{<div id = \"greeting\"><p>\"hello\"</p></div>}}",
    )
    .unwrap();
    assert_eq!(
        formatted(run(project.command(&["--rustfmt"]), "").await),
        ""
    );
    let output = std::fs::read_to_string(path).unwrap();
    assert!(output.starts_with("fn main() {\n"));
    assert!(output.contains("<div id=\"greeting\">"));
}

#[tokio::test]
async fn formatter_errors_leave_files_unchanged_and_emit_no_source() {
    let project = Project::new();
    for input in ["fn main( {", "fn main(){view!{<div></span>}}"] {
        let output = run(project.command(&["--stdin", "--rustfmt"]), input).await;
        assert!(!output.status.success());
        assert_eq!(output.stdout.as_slice(), b"");
        assert_ne!(output.stderr.as_slice(), b"");

        let path = project.path.join("main.rs");
        std::fs::write(&path, input).unwrap();
        let output = run(project.command(&["--rustfmt", "main.rs"]), "").await;
        assert!(!output.status.success());
        assert_eq!(std::fs::read_to_string(path).unwrap(), input);
    }
}

#[tokio::test]
async fn missing_rustfmt_reports_failure_without_emitting_source() {
    let project = Project::new();
    let mut command = project.command(&["--stdin", "--rustfmt"]);
    command.env("PATH", "");
    let output = run(command, "fn main() {}").await;
    assert!(!output.status.success());
    assert_eq!(output.stdout.as_slice(), b"");
    assert!(String::from_utf8_lossy(&output.stderr).contains("rustfmt"));
}

#[tokio::test]
async fn large_stdin_does_not_block_on_process_pipes() {
    let project = Project::new();
    let mut input = "// A comment to fill the process pipes.\n".repeat(16_384);
    input.push_str("fn main(){let x=1;}\n");
    let output = formatted(run(project.command(&["--stdin", "--rustfmt"]), &input).await);
    assert!(output.ends_with("fn main() {\n    let x = 1;\n}\n"));
    assert_eq!(output.matches("// A comment").count(), 16_384);
}

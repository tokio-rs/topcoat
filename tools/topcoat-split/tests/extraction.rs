use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Experiment {
    directory: PathBuf,
}

impl Experiment {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/extraction-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&directory).unwrap();
        Self { directory }
    }

    fn analyze(&self, source: &str) -> Output {
        let input = self.directory.join("server.rs");
        std::fs::write(&input, source).unwrap();
        Command::new(env!("CARGO_BIN_EXE_topcoat-split"))
            .arg("--out-dir")
            .arg(self.directory.join("client"))
            .arg("--")
            .arg(input)
            .arg("--edition=2024")
            .output()
            .unwrap()
    }

    fn source(&self, bundle: &str) -> String {
        std::fs::read_to_string(
            self.directory
                .join("client")
                .join(bundle)
                .join("src/lib.rs"),
        )
        .unwrap()
    }

    fn rustc(&self) -> Command {
        let mut command = Command::new("rustup");
        command.args([
            "run",
            &std::env::var("TOPCOAT_SPLIT_CLIENT_TOOLCHAIN").unwrap_or("stable".into()),
            "rustc",
        ]);
        command
    }

    fn execute(&self, bundle: &str, test: &str) {
        let input = self.directory.join(format!("{bundle}_test.rs"));
        let output = self.directory.join(format!("{bundle}_test"));
        std::fs::write(&input, format!("{}\n{test}", self.source(bundle))).unwrap();
        success(
            self.rustc()
                .arg(input)
                .args(["--edition=2024", "--test"])
                .arg("-o")
                .arg(&output)
                .output()
                .unwrap(),
        );
        success(Command::new(output).output().unwrap());
    }

    fn check_wasm(&self, bundle: &str) {
        success(
            self.rustc()
                .arg(
                    self.directory
                        .join("client")
                        .join(bundle)
                        .join("src/lib.rs"),
                )
                .args([
                    "--edition=2024",
                    "--crate-type=rlib",
                    "--target=wasm32-unknown-unknown",
                ])
                .arg("-o")
                .arg(self.directory.join(format!("{bundle}.rlib")))
                .output()
                .unwrap(),
        );
    }
}

impl Drop for Experiment {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.directory).unwrap();
        }
    }
}

fn success(output: Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

const PRODUCT: &str = include_str!("fixtures/product.rs");

#[test]
fn splits_bundles_infers_captures_and_excludes_server_methods() {
    let experiment = Experiment::new();
    success(experiment.analyze(PRODUCT));
    let page = experiment.source("page");
    let shard = experiment.source("shard");
    for source in [&page, &shard] {
        assert!(!source.contains("load_from_database"));
        assert!(!source.contains("TcpListener"));
        assert!(!source.contains("unused_client_method"));
        assert!(source.contains("product_signal: &crate::Signal<crate::models::Product>"));
    }
    assert!(page.contains("fn discounted"));
    assert!(!shard.contains("fn discounted"));
    assert!(!shard.contains("fn total"));
    experiment.execute("page", r"
        #[test] fn values() {
            let signal = Signal { value: models::Product { price: 4.5, quantity: 2, discount: 0.1 } };
            assert_eq!(__topcoat_expr_0(&signal), 4.5);
            assert_eq!(__topcoat_expr_1(&signal), 8.1);
        }
    ");
    experiment.execute("shard", r"
        #[test] fn values() {
            let signal = Signal { value: models::Product { price: 4.5, quantity: 7, discount: 0.1 } };
            assert_eq!(__topcoat_expr_0(&signal), 7);
        }
    ");
    experiment.check_wasm("page");
    experiment.check_wasm("shard");
    success(experiment.analyze(PRODUCT));
    assert_eq!(page, experiment.source("page"));
    assert_eq!(shard, experiment.source("shard"));
}

#[test]
fn follows_nested_models_and_iterator_closures() {
    let experiment = Experiment::new();
    let source = PRODUCT.replace("pub discount: f64,", "pub discount: f64, pub name: String, pub lines: Vec<Line>,")
        .replace("impl Product {", "pub struct Line { pub amount: f64 }\nimpl Product {")
        .replace("discounted(self.price, self.discount) * self.quantity as f64", "discounted(self.price, self.discount) * self.quantity as f64 + self.lines.iter().map(|line| line.amount).sum::<f64>() + self.name.len() as f64")
        .replace("discount: 0.1 }", "discount: 0.1, name: String::new(), lines: Vec::new() }");
    success(experiment.analyze(&source));
    experiment.execute(
        "page",
        r#"
        #[test] fn values() {
            let signal = Signal { value: models::Product {
                price: 4.5, quantity: 2, discount: 0.1, name: "abc".into(),
                lines: vec![models::Line { amount: 2.0 }, models::Line { amount: 3.0 }],
            }};
            assert_eq!(__topcoat_expr_1(&signal), 16.1);
        }
    "#,
    );
    experiment.check_wasm("page");
}

#[test]
fn rejects_projected_captures_without_partial_output() {
    let experiment = Experiment::new();
    let source = PRODUCT.replace("product_signal.get().price", "product_signal.value.price");
    let output = experiment.analyze(&source);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("projected capture"));
    assert!(!experiment.directory.join("client").exists());
}

#[test]
fn rejects_destructors_instead_of_silently_dropping_behavior() {
    let experiment = Experiment::new();
    let source = PRODUCT.replace(
        "impl Product {",
        "impl Drop for Product { fn drop(&mut self) {} }\nimpl Product {",
    );
    let output = experiment.analyze(&source);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("destructor"));
    assert!(!experiment.directory.join("client").exists());
}

#[test]
fn rejects_attributes_instead_of_changing_type_layout() {
    let experiment = Experiment::new();
    let source = PRODUCT.replace("pub struct Product", "#[repr(C)] pub struct Product");
    let output = experiment.analyze(&source);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("attribute-aware"));
}

#[test]
fn rejects_missing_markers() {
    let experiment = Experiment::new();
    let output = experiment.analyze("fn main() {}");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no __topcoat_split_root"));
}

fn native_source(body: &str) -> String {
    format!(
        r#"
        fn __topcoat_wasm_owner<T>(_: &str, _: &str) {{}}
        fn __topcoat_wasm_root<F>(_: &str, f: F) -> F {{ f }}
        struct Page;
        fn main() {{
            __topcoat_wasm_owner::<Page>("page", "test");
            {body}
        }}
    "#
    )
}

#[test]
fn rejects_lossy_integer_captures_in_native_bridge() {
    let experiment = Experiment::new();
    let source = native_source(
        r##"
        let value = u64::MAX;
        let _ = __topcoat_wasm_root(r#"{"id":"one","handler":false}"#, || value as u32);
    "##,
    );
    let output = experiment.analyze(&source);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("precision loss"));
    assert!(!experiment.directory.join("client").exists());
}

#[test]
fn rejects_mutable_captures_without_writing_partial_bundles() {
    let experiment = Experiment::new();
    let source = native_source(
        r##"
        let mut value = 1i32;
        let _ = __topcoat_wasm_root(r#"{"id":"one","handler":false}"#, || { value += 1; value });
    "##,
    );
    let output = experiment.analyze(&source);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("mutable captures"));
    assert!(!experiment.directory.join("client").exists());
}

#[test]
fn rejects_native_string_and_collection_captures() {
    for initializer in ["String::from(\"hello\")", "vec![1u32, 2]"] {
        let experiment = Experiment::new();
        let source = native_source(&format!(
            r##"
            let value = {initializer};
            let _ = __topcoat_wasm_root(r#"{{"id":"one","handler":false}}"#, || value.len() as u32);
        "##,
        ));
        let output = experiment.analyze(&source);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("use Text, primitives"));
        assert!(!experiment.directory.join("client").exists());
    }
}

#[test]
fn native_primitive_dispatch_has_no_serde_dependency() {
    let experiment = Experiment::new();
    success(experiment.analyze(&native_source(
        r##"
        let value = 12u32;
        let _ = __topcoat_wasm_root(r#"{"id":"one","handler":false}"#, || value + 1);
    "##,
    )));
    let source = experiment.source("test");
    assert!(!source.contains("serde"));
    assert!(!source.contains("-> String"));
    assert!(source.contains("ClientValue>::capture(0)"));
    let cargo =
        std::fs::read_to_string(experiment.directory.join("client/test/Cargo.toml")).unwrap();
    assert!(!cargo.contains("serde"));
}

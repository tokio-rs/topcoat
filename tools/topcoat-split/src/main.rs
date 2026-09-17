#![feature(rustc_private)]
#![doc = include_str!("../docs/split.md")]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

mod split;
mod wasm;

use std::path::PathBuf;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;

struct Analyzer {
    output: PathBuf,
    error: Option<String>,
    continue_build: bool,
}

impl Callbacks for Analyzer {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        if let Err(error) = split::extract(tcx, &self.output) {
            self.error = Some(error);
        }
        if self.continue_build && self.error.is_none() {
            Compilation::Continue
        } else {
            Compilation::Stop
        }
    }
}

fn main() -> std::process::ExitCode {
    rustc_driver::catch_with_exit_code(|| match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("topcoat-split: {error}");
            std::process::ExitCode::FAILURE
        }
    })
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Ok(output) = std::env::var("TOPCOAT_WASM_OUT") {
        let selected = std::env::var("TOPCOAT_WASM_CRATE").map_err(|e| e.to_string())?;
        let crate_name = args
            .windows(2)
            .find(|pair| pair[0] == "--crate-name")
            .map(|pair| &pair[1]);
        if crate_name != Some(&selected) {
            let status = std::process::Command::new(&args[0])
                .args(&args[1..])
                .status()
                .map_err(|e| e.to_string())?;
            return if status.success() {
                Ok(())
            } else {
                Err(format!("dependency compiler failed: {status}"))
            };
        }
        let mut analyzer = Analyzer {
            output: output.into(),
            error: None,
            continue_build: true,
        };
        rustc_driver::run_compiler(&args, &mut analyzer);
        return analyzer.error.map_or(Ok(()), Err);
    }
    let Some(separator) = args.iter().position(|arg| arg == "--") else {
        return Err(
            "usage: topcoat-split --out-dir DIRECTORY -- INPUT.rs [rustc arguments]".into(),
        );
    };
    if separator != 2 || args[0] != "--out-dir" || args.len() <= separator + 1 {
        return Err("expected --out-dir DIRECTORY -- INPUT.rs [rustc arguments]".into());
    }
    let mut compiler = vec!["topcoat-split".into()];
    compiler.extend_from_slice(&args[separator + 1..]);
    if compiler
        .iter()
        .any(|arg| arg == "--target" || arg.starts_with("--target="))
    {
        return Err(
            "analyze the native server; build the generated crate for Wasm separately".into(),
        );
    }
    compiler.extend(["--sysroot".into(), env!("SPLIT_SYSROOT").into()]);
    let mut analyzer = Analyzer {
        output: PathBuf::from(&args[1]),
        error: None,
        continue_build: false,
    };
    rustc_driver::run_compiler(&compiler, &mut analyzer);
    analyzer.error.map_or(Ok(()), Err)
}

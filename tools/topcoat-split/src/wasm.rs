use rustc_hir::{
    self as hir,
    def::Res,
    def_id::DefId,
    intravisit::{self, Visitor},
};
use rustc_middle::ty::{self, TyCtxt};

pub const CARGO: &str = r#"
[lib]
crate-type = ["cdylib", "rlib"]
[dependencies]
wasm-bindgen = "=0.2.128"
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
"#;

pub const HOST: &str = include_str!("wasm/host.js");

pub fn is_signal(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local()
        && tcx.crate_name(id.krate).as_str() == "topcoat_runtime"
        && tcx.item_name(id).as_str() == "Signal"
}

pub fn is_text(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local()
        && tcx.crate_name(id.krate).as_str() == "topcoat_runtime"
        && tcx.item_name(id).as_str() == "Text"
}

pub fn mapped_path(tcx: TyCtxt<'_>, id: DefId) -> Option<String> {
    if is_signal(tcx, id) {
        return Some("crate::__topcoat::Signal".into());
    }
    if is_text(tcx, id) {
        return Some("crate::__topcoat::Text".into());
    }
    let implementation = tcx.impl_of_assoc(id)?;
    if let ty::Adt(definition, _) = tcx
        .type_of(implementation)
        .instantiate_identity()
        .skip_normalization()
        .kind()
    {
        let method = tcx.item_name(id);
        let name = if is_signal(tcx, definition.did()) && matches!(method.as_str(), "get" | "set") {
            "Signal"
        } else if is_text(tcx, definition.did()) && matches!(method.as_str(), "concat" | "is_empty")
        {
            "Text"
        } else {
            return None;
        };
        Some(format!("crate::__topcoat::{name}::{method}"))
    } else {
        None
    }
}

pub fn uses_parameter(expr: &hir::Expr<'_>, parameter: hir::HirId) -> bool {
    struct Uses {
        parameter: hir::HirId,
        found: bool,
    }
    impl<'v> Visitor<'v> for Uses {
        fn visit_path(&mut self, path: &hir::Path<'v>, _: hir::HirId) {
            if matches!(path.res, Res::Local(id) if id == self.parameter) {
                self.found = true;
            }
            intravisit::walk_path(self, path);
        }
    }
    let mut visitor = Uses {
        parameter,
        found: false,
    };
    visitor.visit_expr(expr);
    visitor.found
}

pub fn runtime(manifest: &serde_json::Value) -> Result<String, String> {
    let mut source = include_str!("wasm/runtime.rs.txt").to_owned();
    source.push_str("\n#[wasm_bindgen::prelude::wasm_bindgen]\npub fn dispatch(index: u32) -> wasm_bindgen::JsValue { match index {\n");
    for entry in manifest["entries"].as_array().unwrap() {
        let index = entry["index"].as_u64().unwrap();
        source.push_str(&format!("{index} => {{\n"));
        let mut arguments = Vec::new();
        for (index, capture) in entry["captures"].as_array().unwrap().iter().enumerate() {
            let value_type = capture["value_type"].as_str().unwrap();
            let parameter_type = capture["type"].as_str().unwrap();
            let signal = capture["signal"] == true;
            if parameter_type.contains("&mut ") {
                return Err("mutable captures require an explicit state handle".into());
            }
            if signal {
                let inner = value_type.trim_start_matches('&');
                source.push_str(&format!(
                    "let capture_{index}: {inner} = crate::__topcoat::Signal::new({index});\n"
                ));
                let borrows = parameter_type.chars().take_while(|c| *c == '&').count();
                arguments.push(format!("{}capture_{index}", "&".repeat(borrows)));
            } else {
                let inner = value_type.trim_start_matches('&');
                source.push_str(&format!("let capture_{index}: {inner} = <{inner} as crate::__topcoat::ClientValue>::capture({index});\n"));
                let borrows = parameter_type.chars().take_while(|c| *c == '&').count();
                arguments.push(format!("{}capture_{index}", "&".repeat(borrows)));
            }
        }
        source.push_str(&format!(
            "let result = crate::{}({});\ncrate::__topcoat::ClientValue::into_js(result)\n}},\n",
            entry["function"].as_str().unwrap(),
            arguments.join(", ")
        ));
    }
    source.push_str("_ => core::arch::wasm32::unreachable(),\n}}\n");
    Ok(source)
}

pub fn bridge_type(tcx: TyCtxt<'_>, ty: ty::Ty<'_>) -> Result<String, String> {
    let ty = ty.peel_refs();
    match ty.kind() {
        ty::Bool | ty::Float(rustc_ast::FloatTy::F64) => Ok(ty.to_string()),
        ty::Int(rustc_ast::IntTy::I8 | rustc_ast::IntTy::I16 | rustc_ast::IntTy::I32)
        | ty::Uint(rustc_ast::UintTy::U8 | rustc_ast::UintTy::U16 | rustc_ast::UintTy::U32) => {
            Ok(ty.to_string())
        }
        ty::Int(_) | ty::Uint(_) => Err(format!(
            "`{ty}` is not supported by the typed bridge; integers wider than 32 bits risk precision loss"
        )),
        ty::Tuple(elements) if elements.is_empty() => Ok("()".into()),
        ty::Adt(definition, _) if is_text(tcx, definition.did()) => Ok("Text".into()),
        ty::Adt(definition, args) if is_signal(tcx, definition.did()) => {
            bridge_type(tcx, args.type_at(0))
        }
        _ => Err(format!(
            "`{ty}` is not supported by the typed Wasm bridge; use Text, primitives, or signals of those values"
        )),
    }
}

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
[workspace.dependencies]
wasm-bindgen = { version = "=0.2.128", default-features = false }
wee_alloc = "=0.4.5"
[dependencies]
wasm-bindgen.workspace = true
wee_alloc.workspace = true
[profile.dev]
panic = "abort"
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

pub fn is_event(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local()
        && tcx.crate_name(id.krate).as_str() == "topcoat_runtime"
        && matches!(tcx.item_name(id).as_str(), "Event" | "EventTarget")
}

pub fn mapped_path(tcx: TyCtxt<'_>, id: DefId) -> Option<String> {
    if is_signal(tcx, id) {
        return Some("crate::__topcoat::Signal".into());
    }
    if is_text(tcx, id) {
        return Some("crate::__topcoat::Text".into());
    }
    if is_event(tcx, id) {
        return Some(format!("crate::__topcoat::{}", tcx.item_name(id)));
    }
    let implementation = tcx.impl_of_assoc(id)?;
    if let ty::Adt(definition, _) = tcx
        .type_of(implementation)
        .instantiate_identity()
        .skip_normalization()
        .kind()
    {
        let method = tcx.item_name(id);
        let name = if is_signal(tcx, definition.did())
            && matches!(method.as_str(), "get" | "set" | "increment" | "decrement")
        {
            "Signal"
        } else if is_text(tcx, definition.did()) && matches!(method.as_str(), "concat" | "is_empty")
        {
            "Text"
        } else if is_event(tcx, definition.did()) {
            "Event"
        } else {
            return None;
        };
        Some(format!("crate::__topcoat::{name}::{method}"))
    } else {
        None
    }
}

pub fn uses_event<'tcx>(tcx: TyCtxt<'tcx>, body: &'tcx hir::Body<'tcx>) -> bool {
    struct Uses<'tcx> {
        tcx: TyCtxt<'tcx>,
        found: bool,
    }
    impl<'tcx> Visitor<'tcx> for Uses<'tcx> {
        fn visit_nested_body(&mut self, id: hir::BodyId) {
            self.visit_body(self.tcx.hir_body(id));
        }
        fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
            if !expr.span.from_expansion()
                && matches!(expr.kind, hir::ExprKind::Path(_))
                && let ty::Adt(def, _) = self
                    .tcx
                    .typeck(expr.hir_id.owner.def_id)
                    .expr_ty(expr)
                    .peel_refs()
                    .kind()
                && is_event(self.tcx, def.did())
            {
                self.found = true;
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut uses = Uses { tcx, found: false };
    uses.visit_body(body);
    uses.found
}

pub fn runtime(manifest: &serde_json::Value) -> Result<String, String> {
    let mut source = include_str!("wasm/runtime.rs.txt").to_owned();
    if manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["async"] == true)
    {
        source = source.replace("// ASYNC_RUNTIME", include_str!("wasm/async.rs.txt"));
    }
    source = source.replace("// EVENT_RUNTIME", include_str!("wasm/event.rs.txt"));
    source.push_str("\n#[wasm_bindgen::prelude::wasm_bindgen]\npub fn dispatch(index: u32) -> wasm_bindgen::JsValue { match index {\n");
    for entry in manifest["entries"].as_array().unwrap() {
        let index = entry["index"].as_u64().unwrap();
        let asynchronous = entry["async"] == true;
        source.push_str(&format!("{index} => {{\n"));
        if asynchronous {
            source.push_str("crate::__topcoat::run(async move {\n");
        }
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
        if entry["event"].as_bool() == Some(true) {
            arguments.push(format!(
                "crate::__topcoat::Event::new({})",
                entry["event_mask"]
            ));
        }
        source.push_str(&format!(
            "let result = crate::{}({}){};\ncrate::__topcoat::ClientValue::into_js(result)\n",
            entry["function"].as_str().unwrap(),
            arguments.join(", "),
            if asynchronous { ".await" } else { "" }
        ));
        if asynchronous {
            source.push_str("})\n");
        }
        source.push_str("},\n");
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

pub struct Procedure<'tcx> {
    pub id: String,
    pub args: Vec<ty::Ty<'tcx>>,
    pub output: ty::Ty<'tcx>,
}

pub fn procedure<'tcx>(tcx: TyCtxt<'tcx>, id: DefId) -> Option<Procedure<'tcx>> {
    let local = id.as_local()?;
    if tcx.def_kind(id) != hir::def::DefKind::Fn {
        return None;
    }
    struct Find<'tcx> {
        tcx: TyCtxt<'tcx>,
        found: Option<Procedure<'tcx>>,
    }
    impl<'tcx> Visitor<'tcx> for Find<'tcx> {
        fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
            let typeck = self.tcx.typeck(expr.hir_id.owner.def_id);
            if let hir::ExprKind::Call(callee, args) = expr.kind
                && let hir::ExprKind::Path(ref path) = callee.kind
                && let Res::Def(_, id) = typeck.qpath_res(path, callee.hir_id)
                && self.tcx.item_name(id).as_str() == "__topcoat_wasm_procedure"
                && self.tcx.crate_name(id.krate).as_str() == "topcoat_runtime"
                && let [label, _] = args
                && let hir::ExprKind::Lit(lit) = label.kind
                && let rustc_ast::LitKind::Str(label, _) = lit.node
            {
                let types = typeck.node_args(callee.hir_id);
                if let ty::Tuple(args) = types.type_at(0).kind() {
                    self.found = Some(Procedure {
                        id: label.to_string(),
                        args: args.iter().collect(),
                        output: types.type_at(1),
                    });
                }
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut find = Find { tcx, found: None };
    find.visit_body(tcx.hir_body_owned_by(local));
    find.found
}

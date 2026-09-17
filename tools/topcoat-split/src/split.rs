use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::Path,
};

use rustc_hir as hir;
use rustc_hir::{
    def::{DefKind, Res},
    def_id::{DefId, LocalDefId},
    intravisit::{self, Visitor, VisitorExt},
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use serde_json::json;

struct Root<'tcx> {
    bundle: String,
    metadata: Option<serde_json::Value>,
    closure: &'tcx hir::Closure<'tcx>,
}

struct Owner {
    kind: String,
    name: String,
    ty: DefId,
}

struct Roots<'tcx> {
    tcx: TyCtxt<'tcx>,
    roots: Vec<Root<'tcx>>,
    owners: HashMap<LocalDefId, Owner>,
    references: HashMap<LocalDefId, HashSet<DefId>>,
    error: Option<String>,
}

fn literal(expr: &hir::Expr<'_>) -> Option<String> {
    if let hir::ExprKind::Lit(lit) = expr.kind
        && let rustc_ast::LitKind::Str(value, _) = lit.node
    {
        Some(value.to_string())
    } else {
        None
    }
}

impl<'tcx> Roots<'tcx> {
    fn owner(&self, mut id: LocalDefId) -> Option<LocalDefId> {
        loop {
            if self.owners.contains_key(&id) {
                return Some(id);
            }
            id = self.tcx.opt_parent(id.to_def_id())?.as_local()?;
        }
    }

    fn plans(&self) -> Result<BTreeMap<String, Vec<&Root<'tcx>>>, String> {
        let mut plans = BTreeMap::new();
        let mut by_type = HashMap::new();
        for (id, owner) in &self.owners {
            let previous = by_type.entry(owner.ty).or_insert(*id);
            if owner.kind != "component" {
                *previous = *id;
            }
        }
        let mut edges: HashMap<LocalDefId, HashSet<LocalDefId>> = HashMap::new();
        for (body, references) in &self.references {
            if let Some(owner) = self.owner(*body) {
                for reference in references {
                    if let Some(target) = by_type.get(reference)
                        && self.owners[target].kind == "component"
                    {
                        edges.entry(owner).or_default().insert(*target);
                    }
                }
            }
        }
        for (id, owner) in &self.owners {
            if owner.kind == "component" {
                continue;
            }
            let mut reachable = HashSet::new();
            let mut pending = vec![*id];
            while let Some(current) = pending.pop() {
                if reachable.insert(current)
                    && let Some(children) = edges.get(&current)
                {
                    pending.extend(children);
                }
            }
            let name = owner.name.replace("::", "-");
            let expressions = self
                .roots
                .iter()
                .filter(|root| {
                    root.metadata.is_some()
                        && self
                            .owner(root.closure.def_id)
                            .is_some_and(|id| reachable.contains(&id))
                })
                .collect::<Vec<_>>();
            if !expressions.is_empty() {
                plans.insert(name, expressions);
            }
        }
        for root in &self.roots {
            if let Some(metadata) = &root.metadata {
                if !plans.values().any(|roots| {
                    roots
                        .iter()
                        .any(|other| other.closure.def_id == root.closure.def_id)
                }) {
                    return Err(format!(
                        "Wasm expression {} is not reachable from a page or shard",
                        metadata["id"]
                    ));
                }
            } else {
                plans.entry(root.bundle.clone()).or_default().push(root);
            }
        }
        Ok(plans)
    }
}

impl<'tcx> Visitor<'tcx> for Roots<'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let typeck = self.tcx.typeck(expr.hir_id.owner.def_id);
        let Some(expr_ty) = typeck.expr_ty_opt(expr) else {
            return;
        };
        let references = self.references.entry(expr.hir_id.owner.def_id).or_default();
        for arg in expr_ty.walk().chain(typeck.node_args(expr.hir_id).iter()) {
            if let Some(ty) = arg.as_type()
                && let ty::Adt(def, _) = ty.kind()
            {
                references.insert(def.did());
            }
        }
        if let hir::ExprKind::Call(callee, args) = expr.kind
            && let hir::ExprKind::Path(ref path) = callee.kind
            && let Res::Def(DefKind::Fn, id) = typeck.qpath_res(path, callee.hir_id)
        {
            let name = self.tcx.item_name(id);
            match name.as_str() {
                "__topcoat_wasm_owner" => {
                    if let [kind, name] = args
                        && let (Some(kind), Some(name)) = (literal(kind), literal(name))
                        && let Some(arg) = typeck.node_args(callee.hir_id).first()
                        && let Some(ty) = arg.as_type()
                        && let ty::Adt(def, _) = ty.kind()
                    {
                        let entry =
                            self.owners
                                .entry(expr.hir_id.owner.def_id)
                                .or_insert_with(|| Owner {
                                    kind: kind.clone(),
                                    name: name.clone(),
                                    ty: def.did(),
                                });
                        if kind != "component" {
                            *entry = Owner {
                                kind,
                                name,
                                ty: def.did(),
                            };
                        }
                    } else {
                        self.error = Some("invalid render owner marker".into());
                    }
                }
                "__topcoat_split_root" | "__topcoat_wasm_root" => {
                    if let [label, body] = args
                        && let Some(label) = literal(label)
                        && let hir::ExprKind::Closure(closure) = body.kind
                    {
                        if name.as_str() == "__topcoat_wasm_root" {
                            match serde_json::from_str(&label) {
                                Ok(metadata) => self.roots.push(Root {
                                    bundle: String::new(),
                                    metadata: Some(metadata),
                                    closure,
                                }),
                                Err(error) => self.error = Some(error.to_string()),
                            }
                        } else if !label.is_empty()
                            && label
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                        {
                            self.roots.push(Root {
                                bundle: label,
                                metadata: None,
                                closure,
                            });
                        } else {
                            self.error = Some("invalid bundle name".into());
                        }
                    } else {
                        self.error = Some("a split root requires a literal and closure".into());
                    }
                }
                _ => {}
            }
        }
        intravisit::walk_expr(self, expr);
    }
}

struct Bundle<'tcx> {
    tcx: TyCtxt<'tcx>,
    selected: HashSet<LocalDefId>,
    pending: Vec<LocalDefId>,
    edits: BTreeMap<(u32, u32), String>,
    modules: BTreeMap<Vec<String>, Vec<String>>,
    entries: Vec<serde_json::Value>,
    error: Option<String>,
    native: bool,
    event_mask: u64,
    procedures: BTreeMap<String, serde_json::Value>,
}

impl<'tcx> Bundle<'tcx> {
    fn new(tcx: TyCtxt<'tcx>, native: bool) -> Self {
        Self {
            tcx,
            native,
            event_mask: 0,
            procedures: BTreeMap::new(),
            selected: HashSet::new(),
            pending: Vec::new(),
            edits: BTreeMap::new(),
            modules: BTreeMap::new(),
            entries: Vec::new(),
            error: None,
        }
    }

    fn path(&self, id: DefId) -> String {
        if self.native
            && let Some(path) = crate::wasm::mapped_path(self.tcx, id)
        {
            return path;
        }
        let path = self.tcx.def_path_str(id);
        if id.is_local() {
            format!("crate::{path}")
        } else {
            format!("::{path}")
        }
    }

    fn require(&mut self, mut id: DefId) {
        if self.native
            && let Some(procedure) = crate::wasm::procedure(self.tcx, id)
        {
            let local = id.expect_local();
            if self.procedures.contains_key(&self.tcx.def_path_str(id)) {
                return;
            }
            let result = (|| -> Result<(), String> {
                let index = self.procedures.len();
                let mut params = Vec::new();
                let mut bridges = Vec::new();
                let mut arguments = Vec::new();
                for (index, ty) in procedure.args.iter().enumerate() {
                    bridges.push(crate::wasm::bridge_type(self.tcx, *ty)?);
                    params.push(format!("arg{index}: {}", self.type_source(*ty)?));
                    arguments.push(format!(
                        "args.push(&crate::__topcoat::ClientValue::into_js(arg{index}));"
                    ));
                }
                let bridge = crate::wasm::bridge_type(self.tcx, procedure.output)?;
                let output = self.type_source(procedure.output)?;
                let name = self.tcx.item_name(id);
                self.modules.entry(self.module(local)).or_default().push(format!(
                    "pub fn {name}({}) -> impl core::future::Future<Output = {output}> {{ let args = crate::__topcoat::Arguments::new(); {} crate::__topcoat::procedure({index}, args) }}",
                    params.join(", "), arguments.join("\n")));
                self.procedures.insert(
                    self.tcx.def_path_str(id),
                    json!({"index": index, "id": procedure.id, "args": bridges, "result": bridge}),
                );
                Ok(())
            })();
            if let Err(error) = result {
                self.error = Some(error);
            }
            return;
        }

        if self.native && crate::wasm::mapped_path(self.tcx, id).is_some() {
            return;
        }
        if !id.is_local() {
            let name = self.tcx.crate_name(id.krate);
            if !matches!(name.as_str(), "std" | "core" | "alloc") {
                self.error = Some(format!(
                    "external dependency `{name}` requires an explicit client dependency; automatic external crate extraction is not supported"
                ));
            }
            return;
        }
        if matches!(
            self.tcx.def_kind(id),
            DefKind::Ctor(..) | DefKind::Variant | DefKind::Field
        ) {
            id = self.tcx.parent(id);
            if self.tcx.def_kind(id) == DefKind::Variant {
                id = self.tcx.parent(id);
            }
        }
        if matches!(
            self.tcx.def_kind(id),
            DefKind::Mod
                | DefKind::TyParam
                | DefKind::LifetimeParam
                | DefKind::ConstParam
                | DefKind::Closure
        ) {
            return;
        }
        if matches!(self.tcx.def_kind(id), DefKind::Struct | DefKind::Enum) {
            let definition = self.tcx.adt_def(id);
            if definition.destructor(self.tcx).is_some()
                || definition.async_destructor(self.tcx).is_some()
            {
                self.error = Some(format!(
                    "type `{}` has a destructor; extracting it without its Drop implementation would change behavior",
                    self.tcx.def_path_str(id)
                ));
            }
        }
        let local = id.expect_local();
        if self.selected.insert(local) {
            self.pending.push(local);
        }
    }

    fn require_type(&mut self, ty: Ty<'tcx>) {
        for argument in ty.walk() {
            if let Some(ty) = argument.as_type()
                && let ty::Adt(definition, _) = ty.kind()
            {
                self.require(definition.did());
            }
        }
    }

    fn type_source(&mut self, ty: Ty<'tcx>) -> Result<String, String> {
        self.require_type(ty);
        Ok(match ty.kind() {
            ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => {
                ty.to_string()
            }
            ty::Adt(definition, args) => {
                let path = self.path(definition.did());
                let args = args
                    .iter()
                    .map(|arg| match arg.kind() {
                        ty::GenericArgKind::Type(ty) => self.type_source(ty),
                        _ => {
                            Err("lifetime and const generic captures are not supported yet".into())
                        }
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                if args.is_empty() {
                    path
                } else {
                    format!("{path}<{}>", args.join(", "))
                }
            }
            ty::Ref(_, inner, mutability) => format!(
                "&{}{}",
                if mutability.is_mut() { "mut " } else { "" },
                self.type_source(*inner)?
            ),
            ty::Tuple(elements) => {
                let elements = elements
                    .iter()
                    .map(|ty| self.type_source(ty))
                    .collect::<Result<Vec<_>, _>>()?;
                format!(
                    "({})",
                    elements
                        .iter()
                        .map(|ty| format!("{ty},"))
                        .collect::<String>()
                )
            }
            ty::Slice(inner) => format!("[{}]", self.type_source(*inner)?),
            _ => {
                return Err(format!(
                    "capture/result type `{ty}` cannot yet be emitted as source"
                ));
            }
        })
    }

    fn module(&self, id: LocalDefId) -> Vec<String> {
        let mut module = Vec::new();
        let mut parent = id.to_def_id();
        while let Some(next) = self.tcx.opt_parent(parent) {
            parent = next;
            if self.tcx.def_kind(parent) == DefKind::Mod && !parent.is_crate_root() {
                module.push(self.tcx.item_name(parent).to_string());
            }
        }
        module.reverse();
        module
    }

    fn source(&self, span: Span) -> Result<String, String> {
        if span.from_expansion() && !self.native {
            return Err("macro-generated client items require expansion-aware extraction (not supported yet)".into());
        }
        let mut source = self
            .tcx
            .sess
            .source_map()
            .span_to_snippet(span)
            .map_err(|e| format!("source unavailable: {e:?}"))?;
        let mut last = span.hi().0;
        for (&(lo, hi), replacement) in self.edits.iter().rev() {
            if lo >= span.lo().0 && hi <= span.hi().0 {
                if hi > last {
                    return Err("overlapping source rewrites are not supported".into());
                }
                source.replace_range(
                    (lo - span.lo().0) as usize..(hi - span.lo().0) as usize,
                    replacement,
                );
                last = lo;
            }
        }
        Ok(source)
    }

    fn add_root(&mut self, root: &Root<'tcx>) -> Result<(), String> {
        let body = self.tcx.hir_body(root.closure.body);
        let asynchronous = root.metadata.as_ref().is_some_and(|m| m["async"] == true);
        let handler = root
            .metadata
            .as_ref()
            .is_some_and(|metadata| metadata["handler"] == true);
        if !body.params.is_empty() && !handler {
            return Err("split root closures must have zero arguments; handler adapters are not implemented yet".into());
        }
        let captures = self.tcx.closure_captures(root.closure.def_id);
        let mut params = Vec::new();
        let mut metadata = Vec::new();
        for capture in captures {
            let bridge = if self.native {
                Some(crate::wasm::bridge_type(self.tcx, capture.place.base_ty)?)
            } else {
                None
            };
            if !capture.place.projections.is_empty() {
                return Err(format!(
                    "projected capture `{}` is not supported yet; capture a signal handle instead",
                    capture.to_string(self.tcx)
                ));
            }
            let name = capture.var_ident.to_string();
            let value_type = self.type_source(capture.place.base_ty)?;
            let (prefix, mutable) = match capture.info.capture_kind {
                ty::UpvarCapture::ByRef(ty::BorrowKind::Immutable) => ("&", false),
                ty::UpvarCapture::ByRef(_) => ("&mut ", false),
                ty::UpvarCapture::ByValue | ty::UpvarCapture::ByUse => {
                    ("", capture.mutability.is_mut())
                }
            };
            let ty = format!("{prefix}{value_type}");
            params.push(format!("{}{name}: {ty}", if mutable { "mut " } else { "" }));
            let signal = matches!(capture.place.base_ty.peel_refs().kind(), ty::Adt(def, _) if crate::wasm::is_signal(self.tcx, def.did()));
            metadata.push(json!({"name": name, "type": ty, "value_type": value_type, "signal": signal, "bridge": bridge, "capture": format!("{:?}", capture.info.capture_kind)}));
        }
        let event = handler && crate::wasm::uses_event(self.tcx, body);
        if event {
            let parameter = &body.params[0];
            let hir::PatKind::Binding(_, _, name, _) = parameter.pat.kind else {
                return Err("event handlers require a named argument".into());
            };
            params.push(format!("{name}: crate::__topcoat::Event"));
        }
        let result = if asynchronous {
            let hir::ExprKind::Closure(coroutine) = body.value.kind else {
                return Err("unsupported async handler lowering".into());
            };
            let inner = self.tcx.hir_body(coroutine.body);
            self.tcx.typeck(coroutine.def_id).expr_ty(inner.value)
        } else {
            self.tcx.typeck(root.closure.def_id).expr_ty(body.value)
        };
        if self.native {
            crate::wasm::bridge_type(self.tcx, result)?;
        }
        let result = self.type_source(result)?;
        self.visit_body(body);
        let source = self.source(body.value.span)?;
        let index = self.entries.len();
        let name = format!("__topcoat_expr_{index}");
        let module = self.module(root.closure.def_id);
        self.modules
            .entry(module.clone())
            .or_default()
            .push(format!(
                "{}pub {}fn {name}({}) -> {result} {{ {source} }}",
                if self.native && !asynchronous && event {
                    "#[cfg_attr(not(debug_assertions), inline(always))]\n"
                } else {
                    ""
                },
                if asynchronous { "async " } else { "" },
                params.join(", ")
            ));
        let path = module
            .iter()
            .chain(std::iter::once(&name))
            .cloned()
            .collect::<Vec<_>>()
            .join("::");
        self.entries.push(json!({"index": index, "function": path, "captures": metadata, "result": result, "source": source, "id": root.metadata.as_ref().map(|m| &m["id"]), "handler": handler, "event": event, "event_mask": 0, "async": asynchronous}));
        Ok(())
    }

    fn finish(&mut self) -> Result<String, String> {
        let mut items = Vec::new();
        let mut methods: HashMap<LocalDefId, Vec<LocalDefId>> = HashMap::new();
        while let Some(id) = self.pending.pop() {
            if self
                .tcx
                .hir_attrs(self.tcx.local_def_id_to_hir_id(id))
                .iter()
                .any(|attr| {
                    attr.is_doc_comment().is_none()
                        && !(self.native && attr.has_name(rustc_span::Symbol::intern("derive")))
                })
            {
                return Err(format!(
                    "attributes on `{}` require attribute-aware source extraction",
                    self.tcx.def_path_str(id)
                ));
            }
            match self.tcx.hir_node_by_def_id(id) {
                hir::Node::Item(item) => {
                    match item.kind {
                        hir::ItemKind::Fn { .. }
                        | hir::ItemKind::Struct(..)
                        | hir::ItemKind::Enum(..)
                        | hir::ItemKind::Const(..)
                        | hir::ItemKind::TyAlias(..) => {}
                        _ => {
                            return Err(format!(
                                "unsupported client item `{}` ({:?})",
                                self.tcx.def_path_str(id),
                                self.tcx.def_kind(id)
                            ));
                        }
                    }
                    self.visit_item(item);
                    items.push((id, item.span));
                }
                hir::Node::ImplItem(item) => {
                    let parent = self.tcx.parent(id.to_def_id()).expect_local();
                    let hir::Node::Item(parent_item) = self.tcx.hir_node_by_def_id(parent) else {
                        unreachable!()
                    };
                    let hir::ItemKind::Impl(implementation) = parent_item.kind else {
                        unreachable!()
                    };
                    if implementation.of_trait.is_some() {
                        return Err(format!(
                            "local trait implementation `{}` requires trait-aware extraction",
                            self.tcx.def_path_str(parent)
                        ));
                    }
                    self.visit_ty_unambig(implementation.self_ty);
                    self.visit_generics(implementation.generics);
                    self.visit_impl_item(item);
                    methods.entry(parent).or_default().push(id);
                }
                _ => {
                    return Err(format!(
                        "unsupported dependency `{}`",
                        self.tcx.def_path_str(id)
                    ));
                }
            }
        }
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        if self.native {
            for item in self.tcx.hir_crate_items(()).free_items() {
                let item = self.tcx.hir_item(item);
                if let hir::ItemKind::Impl(implementation) = item.kind
                    && implementation.of_trait.is_some()
                    && !item.span.from_expansion()
                    && let ty::Adt(definition, _) = self
                        .tcx
                        .type_of(item.owner_id.def_id)
                        .instantiate_identity()
                        .skip_normalization()
                        .kind()
                    && definition
                        .did()
                        .as_local()
                        .is_some_and(|id| self.selected.contains(&id))
                {
                    return Err(format!(
                        "manual trait implementation for `{}` needs trait-aware extraction",
                        self.tcx.def_path_str(definition.did())
                    ));
                }
            }
        }
        items.sort_by_key(|(_, span)| span.lo());
        for (id, span) in items {
            let mut source = self.source(span)?;
            if self.native && matches!(self.tcx.def_kind(id), DefKind::Struct | DefKind::Enum) {
                source = format!("#[derive(Clone)]\n{source}");
            }
            self.modules
                .entry(self.module(id))
                .or_default()
                .push(source);
        }
        let mut methods = methods.into_iter().collect::<Vec<_>>();
        methods.sort_by_key(|(parent, _)| self.tcx.def_span(*parent).lo());
        for (parent, mut ids) in methods {
            let hir::Node::Item(item) = self.tcx.hir_node_by_def_id(parent) else {
                unreachable!()
            };
            let hir::ItemKind::Impl(implementation) = item.kind else {
                unreachable!()
            };
            let first = implementation
                .items
                .iter()
                .map(|id| self.tcx.hir_impl_item(*id).span.lo())
                .min()
                .ok_or("empty impl")?;
            let header = self.source(item.span.with_hi(first))?;
            let brace = header
                .rfind('{')
                .ok_or("impl header missing opening brace")?;
            let mut source = header[..=brace].to_string();
            ids.sort_by_key(|id| self.tcx.def_span(*id).lo());
            for id in ids {
                let hir::Node::ImplItem(method) = self.tcx.hir_node_by_def_id(id) else {
                    unreachable!()
                };
                source.push_str(&self.source(method.span)?);
                source.push('\n');
            }
            source.push('}');
            self.modules
                .entry(self.module(parent))
                .or_default()
                .push(source);
        }
        Ok(format!(
            "#![forbid(unsafe_code)]\nextern crate alloc;\n{}",
            self.render_module(&[])
        ))
    }

    fn render_module(&self, prefix: &[String]) -> String {
        let mut source = self
            .modules
            .get(prefix)
            .map(|items| items.join("\n\n"))
            .unwrap_or_default();
        let children = self
            .modules
            .keys()
            .filter(|key| key.starts_with(prefix) && key.len() > prefix.len())
            .map(|key| key[prefix.len()].clone())
            .collect::<BTreeSet<_>>();
        for child in children {
            let mut path = prefix.to_vec();
            path.push(child.clone());
            source.push_str(&format!(
                "\npub mod {child} {{\n{}\n}}\n",
                self.render_module(&path)
            ));
        }
        source
    }
}

impl<'tcx> Visitor<'tcx> for Bundle<'tcx> {
    fn visit_nested_body(&mut self, body: hir::BodyId) {
        self.visit_body(self.tcx.hir_body(body));
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let typeck = self.tcx.typeck(expr.hir_id.owner.def_id);
        if self.native
            && let hir::ExprKind::Field(base, field) = expr.kind
            && let ty::Adt(def, _) = typeck.expr_ty(base).peel_refs().kind()
            && crate::wasm::is_event(self.tcx, def.did())
        {
            let fields: Vec<String> =
                serde_json::from_str(include_str!("wasm/event-fields.json")).unwrap();
            for (index, name) in fields.iter().enumerate() {
                if name.split('.').next_back() == Some(field.as_str()) {
                    self.event_mask |= 1 << index;
                }
            }
        }
        self.require_type(typeck.expr_ty(expr));
        if let Some(id) = typeck.type_dependent_def_id(expr.hir_id) {
            if self.tcx.trait_of_assoc(id).is_some() {
                // Standard-library traits remain available in the client. Local
                // implementations need explicit collection before we can emit them.
                if let hir::ExprKind::MethodCall(_, receiver, _, _) = expr.kind
                    && typeck.expr_ty(receiver).peel_refs().is_adt()
                    && let ty::Adt(def, _) = typeck.expr_ty(receiver).peel_refs().kind()
                    && def.did().is_local()
                {
                    self.error = Some(format!(
                        "trait call `{}` on a local type requires trait-aware extraction",
                        self.tcx.def_path_str(id)
                    ));
                }
            }
            self.require(id);
        }
        intravisit::walk_expr(self, expr);
    }

    fn visit_path(&mut self, path: &hir::Path<'tcx>, _id: hir::HirId) {
        if let Res::Def(kind, definition) = path.res {
            self.require(definition);
            if !matches!(
                kind,
                DefKind::TyParam | DefKind::ConstParam | DefKind::LifetimeParam
            ) && !path.segments.is_empty()
                && !path.span.from_expansion()
            {
                if path.segments[..path.segments.len() - 1]
                    .iter()
                    .any(|segment| segment.args.is_some())
                {
                    self.error = Some(
                        "generic arguments on intermediate path segments are not supported yet"
                            .into(),
                    );
                } else {
                    let last = path.segments.last().unwrap();
                    // Retain generic arguments on the last segment.
                    self.edits.insert(
                        (path.span.lo().0, last.ident.span.hi().0),
                        self.path(definition),
                    );
                }
            }
        }
        intravisit::walk_path(self, path);
    }
}

pub fn extract(tcx: TyCtxt<'_>, output: &Path) -> Result<(), String> {
    let mut roots = Roots {
        tcx,
        roots: Vec::new(),
        error: None,
        owners: HashMap::new(),
        references: HashMap::new(),
    };
    for owner in tcx.hir_body_owners() {
        roots.visit_body(tcx.hir_body_owned_by(owner));
    }
    if let Some(error) = roots.error {
        return Err(error);
    }
    if roots.roots.is_empty() {
        return Err("no __topcoat_split_root(bundle, closure) markers found".into());
    }
    roots
        .roots
        .sort_by_key(|root| tcx.def_span(root.closure.def_id).lo());
    let mut bundles = BTreeMap::new();
    let native = roots.roots.iter().any(|root| root.metadata.is_some());
    for (name, expressions) in roots.plans()? {
        let bundle = bundles
            .entry(name)
            .or_insert_with(|| Bundle::new(tcx, native));
        for expression in expressions {
            bundle.add_root(expression)?;
        }
    }
    // Finish every analysis before writing any generated files.
    let mut artifacts = Vec::new();
    for (name, mut bundle) in bundles {
        let mut source = bundle
            .finish()
            .map_err(|error| format!("bundle `{name}`: {error}"))?;
        for entry in &mut bundle.entries {
            entry["event_mask"] = json!(bundle.event_mask);
        }
        let mut items = bundle
            .selected
            .iter()
            .map(|id| tcx.def_path_str(*id))
            .collect::<Vec<_>>();
        items.sort();
        let manifest = json!({"version": 1, "bundle": name, "entries": bundle.entries, "items": items, "procedures": bundle.procedures.values().collect::<Vec<_>>()});
        if native {
            source.insert_str(0, "#![no_std]\n");
            source.push_str(include_str!("wasm/allocator.rs.txt"));
            source.push_str(&crate::wasm::runtime(&manifest)?);
        }
        artifacts.push((name, source, manifest));
    }
    let mut index = json!({"version": 1, "bundles": [], "expressions": {}});
    for (name, source, manifest) in artifacts {
        let directory = output.join(&name);
        std::fs::create_dir_all(directory.join("src")).map_err(|e| e.to_string())?;
        std::fs::write(directory.join("src/lib.rs"), source).map_err(|e| e.to_string())?;
        std::fs::write(directory.join("Cargo.toml"), format!("[workspace]\n[package]\nname = \"topcoat-client-{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n")).map_err(|e| e.to_string())?;
        std::fs::write(
            directory.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        if native {
            use std::io::Write;
            let mut cargo = std::fs::OpenOptions::new()
                .append(true)
                .open(directory.join("Cargo.toml"))
                .map_err(|e| e.to_string())?;
            cargo
                .write_all(crate::wasm::CARGO.as_bytes())
                .map_err(|e| e.to_string())?;
            std::fs::write(directory.join("host.js"), crate::wasm::HOST)
                .map_err(|e| e.to_string())?;
        }
        index["bundles"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": name, "entries": manifest["entries"], "procedures": manifest["procedures"]}));
        for entry in manifest["entries"].as_array().unwrap() {
            if let Some(id) = entry["id"].as_str() {
                index["expressions"][id] = entry.clone();
            }
        }
        println!("{}", directory.display());
    }
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_string_pretty(&index).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

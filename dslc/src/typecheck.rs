use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Def, Expr, Field, FnDef, MapKind, MatchArm, MatchPat, StructDef, Top, Ty, UnionDef, VariantDef,
};
use crate::diag::{Diag, DslResult, Span};
use crate::typed::{CastHint, GenericBound, ParamMode, SpanKey, TypedDef, TypedExpr, TypedFn};

#[derive(Debug, Clone)]
pub struct FnSig {
    pub params: Vec<Ty>,
    pub param_modes: Vec<ParamMode>,
    pub ret: Ty,
}

#[derive(Debug, Clone)]
pub struct FnEnv {
    pub fns: BTreeMap<String, Vec<FnSig>>,
}

impl FnEnv {
    pub fn new() -> Self {
        Self {
            fns: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, sig: FnSig) -> DslResult<()> {
        let entry = self.fns.entry(name.clone()).or_default();
        if entry.iter().any(|existing| existing.params.len() == sig.params.len()) {
            return Err(Diag::new(format!(
                "duplicate function '{}' with arity {}",
                name,
                sig.params.len()
            )));
        }
        entry.push(sig);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&[FnSig]> {
        self.fns.get(name).map(|v| v.as_slice())
    }

    pub fn get_arity(&self, name: &str, arity: usize) -> Option<&FnSig> {
        self.fns
            .get(name)
            .and_then(|sigs| sigs.iter().find(|sig| sig.params.len() == arity))
    }
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub structs: BTreeMap<String, StructDef>,
    pub unions: BTreeMap<String, UnionDef>,
    pub variants: BTreeMap<String, (String, VariantDef)>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            structs: BTreeMap::new(),
            unions: BTreeMap::new(),
            variants: BTreeMap::new(),
        }
    }

    pub fn insert_struct(&mut self, sd: StructDef) -> DslResult<()> {
        if self.structs.contains_key(&sd.name)
            || self.unions.contains_key(&sd.name)
            || self.variants.contains_key(&sd.name)
        {
            return Err(Diag::new(format!("duplicate struct '{}'", sd.name)).with_span(sd.span));
        }
        self.structs.insert(sd.name.clone(), sd);
        Ok(())
    }

    pub fn insert_union(&mut self, ud: UnionDef) -> DslResult<()> {
        if self.structs.contains_key(&ud.name)
            || self.unions.contains_key(&ud.name)
            || self.variants.contains_key(&ud.name)
        {
            return Err(Diag::new(format!("duplicate union '{}'", ud.name)).with_span(ud.span));
        }
        for v in &ud.variants {
            if self.structs.contains_key(&v.name)
                || self.unions.contains_key(&v.name)
                || self.variants.contains_key(&v.name)
            {
                return Err(
                    Diag::new(format!("duplicate variant '{}'", v.name)).with_span(v.span.clone())
                );
            }
            self.variants
                .insert(v.name.clone(), (ud.name.clone(), v.clone()));
        }
        self.unions.insert(ud.name.clone(), ud);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TypecheckedProgram {
    pub env: TypeEnv,
    pub typed_fns: Vec<TypedFn>,
    pub typed_defs: Vec<TypedDef>,
    pub fn_env: FnEnv,
    pub auto_clones: BTreeSet<SpanKey>,
    pub warnings: Vec<Diag>,
}

#[derive(Debug, Clone)]
struct InferFnSig {
    params: Vec<InferTy>,
    ret: InferTy,
}

#[derive(Debug, Clone)]
struct InferFnEnv {
    fns: BTreeMap<String, Vec<InferFnSig>>,
}

impl InferFnEnv {
    fn new() -> Self {
        Self {
            fns: BTreeMap::new(),
        }
    }

    fn insert(&mut self, name: String, sig: InferFnSig) -> DslResult<()> {
        let entry = self.fns.entry(name.clone()).or_default();
        if entry.iter().any(|existing| existing.params.len() == sig.params.len()) {
            return Err(Diag::new(format!(
                "duplicate function '{}' with arity {}",
                name,
                sig.params.len()
            )));
        }
        entry.push(sig);
        Ok(())
    }

    fn get(&self, name: &str) -> Option<&[InferFnSig]> {
        self.fns.get(name).map(|v| v.as_slice())
    }

    fn get_arity(&self, name: &str, arity: usize) -> Option<&InferFnSig> {
        self.fns
            .get(name)
            .and_then(|sigs| sigs.iter().find(|sig| sig.params.len() == arity))
    }
}

fn freshen_infer_ty(ctx: &mut InferCtx, ty: &InferTy, map: &mut BTreeMap<u32, InferTy>) -> InferTy {
    match ctx.resolve(ty) {
        InferTy::Var(id) => map.entry(id).or_insert_with(|| ctx.fresh_var()).clone(),
        InferTy::Vec(inner) => InferTy::Vec(Box::new(freshen_infer_ty(ctx, &inner, map))),
        InferTy::Set(inner) => InferTy::Set(Box::new(freshen_infer_ty(ctx, &inner, map))),
        InferTy::Option(inner) => InferTy::Option(Box::new(freshen_infer_ty(ctx, &inner, map))),
        InferTy::Result(ok, err) => InferTy::Result(
            Box::new(freshen_infer_ty(ctx, &ok, map)),
            Box::new(freshen_infer_ty(ctx, &err, map)),
        ),
        InferTy::Map(kind, k, v) => InferTy::Map(
            kind,
            Box::new(freshen_infer_ty(ctx, &k, map)),
            Box::new(freshen_infer_ty(ctx, &v, map)),
        ),
        InferTy::Fn(params, ret) => InferTy::Fn(
            params
                .iter()
                .map(|p| freshen_infer_ty(ctx, p, map))
                .collect(),
            Box::new(freshen_infer_ty(ctx, &ret, map)),
        ),
        InferTy::Named(n) => InferTy::Named(n),
    }
}

fn instantiate_sig(ctx: &mut InferCtx, sig: &InferFnSig) -> InferFnSig {
    let mut map = BTreeMap::new();
    let params = sig
        .params
        .iter()
        .map(|t| freshen_infer_ty(ctx, t, &mut map))
        .collect();
    let ret = freshen_infer_ty(ctx, &sig.ret, &mut map);
    InferFnSig { params, ret }
}

#[derive(Debug, Clone)]
struct FieldInfer {
    rust_name: String,
    ty: InferTy,
    span: Span,
}

#[derive(Debug, Clone)]
struct StructInfer {
    fields: Vec<FieldInfer>,
}

#[derive(Debug, Clone)]
struct VariantInfer {
    fields: Vec<FieldInfer>,
}

#[derive(Debug, Clone)]
struct FieldInferEnv {
    structs: BTreeMap<String, StructInfer>,
    variants: BTreeMap<String, VariantInfer>,
}

#[derive(Debug, Clone)]
struct InferFnState {
    def: FnDef,
    param_tys: BTreeMap<String, InferTy>,
    body: InferExpr,
}

#[derive(Debug, Clone)]
struct InferDefState {
    def: Def,
    body: InferExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InferTy {
    Var(u32),
    Named(String),
    Vec(Box<InferTy>),
    Set(Box<InferTy>),
    Option(Box<InferTy>),
    Result(Box<InferTy>, Box<InferTy>),
    Map(MapKind, Box<InferTy>, Box<InferTy>),
    Fn(Vec<InferTy>, Box<InferTy>),
}

#[derive(Debug, Clone)]
struct InferExpr {
    expr: Expr,
    ty: InferTy,
    casts: Vec<CastHint>,
    types: BTreeMap<SpanKey, InferTy>,
}

struct InferCtx {
    next_var: u32,
    subs: BTreeMap<u32, InferTy>,
    numeric_constraints: Vec<(InferTy, Span)>,
    int_constraints: Vec<(InferTy, Span)>,
}

impl InferCtx {
    fn new() -> Self {
        Self {
            next_var: 0,
            subs: BTreeMap::new(),
            numeric_constraints: Vec::new(),
            int_constraints: Vec::new(),
        }
    }

    fn fresh_var(&mut self) -> InferTy {
        let id = self.next_var;
        self.next_var += 1;
        InferTy::Var(id)
    }

    fn resolve(&self, ty: &InferTy) -> InferTy {
        match ty {
            InferTy::Var(id) => match self.subs.get(id) {
                Some(t) => self.resolve(t),
                None => InferTy::Var(*id),
            },
            InferTy::Vec(inner) => InferTy::Vec(Box::new(self.resolve(inner))),
            InferTy::Set(inner) => InferTy::Set(Box::new(self.resolve(inner))),
            InferTy::Option(inner) => InferTy::Option(Box::new(self.resolve(inner))),
            InferTy::Result(ok, err) => {
                InferTy::Result(Box::new(self.resolve(ok)), Box::new(self.resolve(err)))
            }
            InferTy::Map(kind, k, v) => InferTy::Map(
                kind.clone(),
                Box::new(self.resolve(k)),
                Box::new(self.resolve(v)),
            ),
            InferTy::Fn(params, ret) => InferTy::Fn(
                params.iter().map(|p| self.resolve(p)).collect(),
                Box::new(self.resolve(ret)),
            ),
            InferTy::Named(n) => InferTy::Named(n.clone()),
        }
    }

    fn occurs(&self, id: u32, ty: &InferTy) -> bool {
        match self.resolve(ty) {
            InferTy::Var(other) => other == id,
            InferTy::Vec(inner) => self.occurs(id, &inner),
            InferTy::Set(inner) => self.occurs(id, &inner),
            InferTy::Option(inner) => self.occurs(id, &inner),
            InferTy::Result(ok, err) => self.occurs(id, &ok) || self.occurs(id, &err),
            InferTy::Map(_, k, v) => self.occurs(id, &k) || self.occurs(id, &v),
            InferTy::Fn(params, ret) => {
                params.iter().any(|p| self.occurs(id, p)) || self.occurs(id, &ret)
            }
            InferTy::Named(_) => false,
        }
    }

    fn unify(&mut self, a: &InferTy, b: &InferTy, span: &Span) -> DslResult<()> {
        let a = self.resolve(a);
        let b = self.resolve(b);
        match (a, b) {
            (InferTy::Var(id), t) | (t, InferTy::Var(id)) => {
                if matches!(t, InferTy::Var(other) if other == id) {
                    return Ok(());
                }
                if self.occurs(id, &t) {
                    return Err(Diag::new("type inference produced a recursive type")
                        .with_span(span.clone()));
                }
                self.subs.insert(id, t);
                Ok(())
            }
            (InferTy::Named(a), InferTy::Named(b)) => {
                if a == b {
                    Ok(())
                } else {
                    Err(Diag::new(format!("type mismatch: '{}' vs '{}'", a, b))
                        .with_span(span.clone()))
                }
            }
            (InferTy::Vec(a), InferTy::Vec(b)) => self.unify(&a, &b, span),
            (InferTy::Set(a), InferTy::Set(b)) => self.unify(&a, &b, span),
            (InferTy::Option(a), InferTy::Option(b)) => self.unify(&a, &b, span),
            (InferTy::Result(a_ok, a_err), InferTy::Result(b_ok, b_err)) => {
                self.unify(&a_ok, &b_ok, span)?;
                self.unify(&a_err, &b_err, span)
            }
            (InferTy::Map(ka, a_k, a_v), InferTy::Map(kb, b_k, b_v)) => {
                if ka != kb {
                    return Err(
                        Diag::new("type mismatch: map kind differs").with_span(span.clone())
                    );
                }
                self.unify(&a_k, &b_k, span)?;
                self.unify(&a_v, &b_v, span)
            }
            (InferTy::Fn(a_params, a_ret), InferTy::Fn(b_params, b_ret)) => {
                if a_params.len() != b_params.len() {
                    return Err(
                        Diag::new("type mismatch: function arity differs").with_span(span.clone())
                    );
                }
                for (a, b) in a_params.iter().zip(b_params.iter()) {
                    self.unify(a, b, span)?;
                }
                self.unify(&a_ret, &b_ret, span)
            }
            (InferTy::Vec(_), InferTy::Named(_)) | (InferTy::Named(_), InferTy::Vec(_)) => {
                Err(Diag::new("type mismatch: vector vs scalar").with_span(span.clone()))
            }
            (InferTy::Vec(_), InferTy::Set(_)) | (InferTy::Set(_), InferTy::Vec(_)) => {
                Err(Diag::new("type mismatch: vector vs set").with_span(span.clone()))
            }
            (InferTy::Set(_), InferTy::Named(_)) | (InferTy::Named(_), InferTy::Set(_)) => {
                Err(Diag::new("type mismatch: set vs scalar").with_span(span.clone()))
            }
            (InferTy::Option(_), _) | (_, InferTy::Option(_)) => {
                Err(Diag::new("type mismatch: option vs scalar").with_span(span.clone()))
            }
            (InferTy::Result(_, _), _) | (_, InferTy::Result(_, _)) => {
                Err(Diag::new("type mismatch: result vs scalar").with_span(span.clone()))
            }
            (InferTy::Map(_, _, _), _) | (_, InferTy::Map(_, _, _)) => {
                Err(Diag::new("type mismatch: map vs scalar").with_span(span.clone()))
            }
            (InferTy::Fn(_, _), _) | (_, InferTy::Fn(_, _)) => {
                Err(Diag::new("type mismatch: function vs scalar").with_span(span.clone()))
            }
        }
    }
}

fn infer_from_ty(ctx: &mut InferCtx, ty: &Ty) -> InferTy {
    let mut generics = BTreeMap::new();
    infer_from_ty_with_generics(ctx, ty, &mut generics)
}

fn infer_from_ty_with_generics(
    ctx: &mut InferCtx,
    ty: &Ty,
    generics: &mut BTreeMap<u32, InferTy>,
) -> InferTy {
    match ty {
        Ty::Named(n) => InferTy::Named(n.clone()),
        Ty::Vec(inner) => InferTy::Vec(Box::new(infer_from_ty_with_generics(ctx, inner, generics))),
        Ty::Set(inner) => InferTy::Set(Box::new(infer_from_ty_with_generics(ctx, inner, generics))),
        Ty::Option(inner) => {
            InferTy::Option(Box::new(infer_from_ty_with_generics(ctx, inner, generics)))
        }
        Ty::Result(ok, err) => InferTy::Result(
            Box::new(infer_from_ty_with_generics(ctx, ok, generics)),
            Box::new(infer_from_ty_with_generics(ctx, err, generics)),
        ),
        Ty::Map(kind, k, v) => InferTy::Map(
            kind.clone(),
            Box::new(infer_from_ty_with_generics(ctx, k, generics)),
            Box::new(infer_from_ty_with_generics(ctx, v, generics)),
        ),
        Ty::Union(_) => InferTy::Named("__DarcyUnion".to_string()),
        Ty::Generic(id) => generics
            .entry(*id)
            .or_insert_with(|| ctx.fresh_var())
            .clone(),
        Ty::Unknown => ctx.fresh_var(),
    }
}

fn infer_to_ty(ctx: &InferCtx, ty: &InferTy) -> Option<Ty> {
    match ctx.resolve(ty) {
        InferTy::Var(_) => None,
        InferTy::Named(n) => Some(Ty::Named(n)),
        InferTy::Vec(inner) => infer_to_ty(ctx, &inner).map(|t| Ty::Vec(Box::new(t))),
        InferTy::Set(inner) => infer_to_ty(ctx, &inner).map(|t| Ty::Set(Box::new(t))),
        InferTy::Option(inner) => {
            let inner = infer_to_ty(ctx, &inner).unwrap_or(Ty::Unknown);
            Some(Ty::Option(Box::new(inner)))
        }
        InferTy::Result(ok, err) => {
            let ok = infer_to_ty(ctx, &ok).unwrap_or(Ty::Unknown);
            let err = infer_to_ty(ctx, &err).unwrap_or(Ty::Unknown);
            Some(Ty::Result(Box::new(ok), Box::new(err)))
        }
        InferTy::Map(kind, k, v) => {
            let k = infer_to_ty(ctx, &k).unwrap_or(Ty::Unknown);
            let v = infer_to_ty(ctx, &v).unwrap_or(Ty::Unknown);
            Some(Ty::Map(kind, Box::new(k), Box::new(v)))
        }
        InferTy::Fn(_, _) => None,
    }
}

fn infer_to_ty_allow_generic(ctx: &InferCtx, ty: &InferTy) -> Ty {
    match ctx.resolve(ty) {
        InferTy::Var(id) => Ty::Generic(id),
        InferTy::Named(n) => Ty::Named(n),
        InferTy::Vec(inner) => Ty::Vec(Box::new(infer_to_ty_allow_generic(ctx, &inner))),
        InferTy::Set(inner) => Ty::Set(Box::new(infer_to_ty_allow_generic(ctx, &inner))),
        InferTy::Option(inner) => Ty::Option(Box::new(infer_to_ty_allow_generic(ctx, &inner))),
        InferTy::Result(ok, err) => Ty::Result(
            Box::new(infer_to_ty_allow_generic(ctx, &ok)),
            Box::new(infer_to_ty_allow_generic(ctx, &err)),
        ),
        InferTy::Map(kind, k, v) => Ty::Map(
            kind,
            Box::new(infer_to_ty_allow_generic(ctx, &k)),
            Box::new(infer_to_ty_allow_generic(ctx, &v)),
        ),
        InferTy::Fn(_, _) => Ty::Unknown,
    }
}

fn is_concrete_ty(ty: &Ty) -> bool {
    match ty {
        Ty::Unknown => false,
        Ty::Generic(_) => false,
        Ty::Named(_) => true,
        Ty::Vec(inner) => is_concrete_ty(inner),
        Ty::Set(inner) => is_concrete_ty(inner),
        Ty::Option(inner) => is_concrete_ty(inner),
        Ty::Result(ok, err) => is_concrete_ty(ok) && is_concrete_ty(err),
        Ty::Map(_, k, v) => is_concrete_ty(k) && is_concrete_ty(v),
        Ty::Union(items) => items.iter().all(is_concrete_ty),
    }
}

fn infer_ty_rust(ctx: &InferCtx, ty: &InferTy) -> String {
    match ctx.resolve(ty) {
        InferTy::Var(id) => format!("'t{}", id),
        InferTy::Named(n) => n,
        InferTy::Vec(inner) => format!("Arc<Vec<{}>>", infer_ty_rust(ctx, &inner)),
        InferTy::Set(inner) => format!("HashSet<{}>", infer_ty_rust(ctx, &inner)),
        InferTy::Option(inner) => format!("Option<{}>", infer_ty_rust(ctx, &inner)),
        InferTy::Result(ok, err) => {
            format!(
                "Result<{}, {}>",
                infer_ty_rust(ctx, &ok),
                infer_ty_rust(ctx, &err)
            )
        }
        InferTy::Map(kind, k, v) => {
            let name = match kind {
                MapKind::Hash => "HashMap",
                MapKind::BTree => "BTreeMap",
            };
            format!(
                "{}<{}, {}>",
                name,
                infer_ty_rust(ctx, &k),
                infer_ty_rust(ctx, &v)
            )
        }
        InferTy::Fn(params, ret) => {
            let args = params
                .iter()
                .map(|p| infer_ty_rust(ctx, p))
                .collect::<Vec<_>>();
            format!("fn({}) -> {}", args.join(", "), infer_ty_rust(ctx, &ret))
        }
    }
}

fn build_field_infer_env(env: &TypeEnv, ctx: &mut InferCtx) -> DslResult<FieldInferEnv> {
    let mut structs = BTreeMap::new();
    let mut variants = BTreeMap::new();

    for sd in env.structs.values() {
        if sd.extern_ {
            for f in &sd.fields {
                if matches!(f.ty, Ty::Unknown) {
                    return Err(Diag::new(format!(
                        "extern struct '{}' must declare a type for field '{}'",
                        sd.name, f.name
                    ))
                    .with_span(f.span.clone()));
                }
            }
        }
        let fields = sd
            .fields
            .iter()
            .map(|f| FieldInfer {
                rust_name: f.rust_name.clone(),
                ty: infer_from_ty(ctx, &f.ty),
                span: f.span.clone(),
            })
            .collect();
        structs.insert(sd.name.clone(), StructInfer { fields });
    }

    for ud in env.unions.values() {
        if ud.extern_ {
            for v in &ud.variants {
                for f in &v.fields {
                    if matches!(f.ty, Ty::Unknown) {
                        return Err(Diag::new(format!(
                            "extern union '{}' must declare a type for field '{}'",
                            ud.name, f.name
                        ))
                        .with_span(f.span.clone()));
                    }
                }
            }
        }
        for v in &ud.variants {
            let fields = v
                .fields
                .iter()
                .map(|f| FieldInfer {
                    rust_name: f.rust_name.clone(),
                    ty: infer_from_ty(ctx, &f.ty),
                    span: f.span.clone(),
                })
                .collect();
            variants.insert(v.name.clone(), VariantInfer { fields });
        }
    }

    Ok(FieldInferEnv { structs, variants })
}

fn apply_field_inference(
    env: &TypeEnv,
    field_env: &FieldInferEnv,
    ctx: &InferCtx,
) -> DslResult<TypeEnv> {
    let mut out = TypeEnv::new();

    for sd in env.structs.values() {
        let sinfo = field_env.structs.get(&sd.name).ok_or_else(|| {
            Diag::new("internal error: missing struct inference").with_span(sd.span.clone())
        })?;
        let mut fields = Vec::new();
        for (idx, f) in sd.fields.iter().enumerate() {
            let finfo = sinfo.fields.get(idx).ok_or_else(|| {
                Diag::new("internal error: struct field inference mismatch")
                    .with_span(f.span.clone())
            })?;
            let ty = infer_to_ty(ctx, &finfo.ty).ok_or_else(|| {
                Diag::new(format!(
                    "cannot infer type for field '{}.{}'",
                    sd.name, f.name
                ))
                .with_span(finfo.span.clone())
            })?;
            if !is_concrete_ty(&ty) {
                return Err(Diag::new(format!(
                    "cannot infer type for field '{}.{}'",
                    sd.name, f.name
                ))
                .with_span(finfo.span.clone()));
            }
            fields.push(Field {
                name: f.name.clone(),
                rust_name: f.rust_name.clone(),
                ty,
                span: f.span.clone(),
            });
        }
        out.insert_struct(StructDef {
            name: sd.name.clone(),
            rust_name: sd.rust_name.clone(),
            fields,
            span: sd.span.clone(),
            extern_: sd.extern_,
        })?;
    }

    for ud in env.unions.values() {
        let mut variants = Vec::new();
        for v in &ud.variants {
            let vinf = field_env.variants.get(&v.name).ok_or_else(|| {
                Diag::new("internal error: missing variant inference").with_span(v.span.clone())
            })?;
            let mut fields = Vec::new();
            for (idx, f) in v.fields.iter().enumerate() {
                let finfo = vinf.fields.get(idx).ok_or_else(|| {
                    Diag::new("internal error: variant field inference mismatch")
                        .with_span(f.span.clone())
                })?;
                let ty = infer_to_ty(ctx, &finfo.ty).ok_or_else(|| {
                    Diag::new(format!(
                        "cannot infer type for field '{}.{}'",
                        v.name, f.name
                    ))
                    .with_span(finfo.span.clone())
                })?;
                if !is_concrete_ty(&ty) {
                    return Err(Diag::new(format!(
                        "cannot infer type for field '{}.{}'",
                        v.name, f.name
                    ))
                    .with_span(finfo.span.clone()));
                }
                fields.push(Field {
                    name: f.name.clone(),
                    rust_name: f.rust_name.clone(),
                    ty,
                    span: f.span.clone(),
                });
            }
            variants.push(VariantDef {
                name: v.name.clone(),
                rust_name: v.rust_name.clone(),
                fields,
                span: v.span.clone(),
            });
        }
        out.insert_union(UnionDef {
            name: ud.name.clone(),
            rust_name: ud.rust_name.clone(),
            variants,
            span: ud.span.clone(),
            extern_: ud.extern_,
        })?;
    }

    Ok(out)
}

pub fn typecheck_tops(tops: &[Top]) -> DslResult<TypecheckedProgram> {
    let filtered = expand_inline_tops(tops)?;
    let mut env = TypeEnv::new();
    for t in &filtered {
        if let Top::Struct(sd) = t {
            env.insert_struct(sd.clone())?;
        }
        if let Top::Union(ud) = t {
            env.insert_union(ud.clone())?;
        }
    }

    let def_base_names = collect_def_base_names(&filtered);
    let mut ctx = InferCtx::new();
    let field_env = build_field_infer_env(&env, &mut ctx)?;
    let mut fn_env = InferFnEnv::new();
    let mut global_defs: BTreeMap<String, InferTy> = BTreeMap::new();

    for t in &filtered {
        match t {
            Top::Func(fd) => {
                if env.structs.contains_key(&fd.name)
                    || env.unions.contains_key(&fd.name)
                    || env.variants.contains_key(&fd.name)
                    || fn_env.get_arity(&fd.name, fd.params.len()).is_some()
                    || global_defs.contains_key(&fd.name)
                {
                    return Err(Diag::new(format!(
                        "duplicate function '{}' with arity {}",
                        fd.name,
                        fd.params.len()
                    ))
                        .with_span(fd.span.clone()));
                }
                check_param_bindings(fd, &def_base_names)?;
                let mut generics = BTreeMap::new();
                let mut params = Vec::new();
                for p in &fd.params {
                    let ty = match &p.ann {
                        Some(ann) => infer_from_ty_with_generics(&mut ctx, ann, &mut generics),
                        None => ctx.fresh_var(),
                    };
                    params.push(ty);
                }
                if fd.extern_ {
                    for p in &fd.params {
                        if p.ann.is_none() {
                            return Err(Diag::new(format!(
                                "extern function parameter '{}' must declare a type",
                                p.name
                            ))
                            .with_span(p.span.clone()));
                        }
                    }
                }
                let ret = if fd.extern_ {
                    let ret = fd.extern_ret.clone().ok_or_else(|| {
                        Diag::new("extern function must declare return type")
                            .with_span(fd.span.clone())
                    })?;
                    infer_from_ty_with_generics(&mut ctx, &ret, &mut generics)
                } else {
                    ctx.fresh_var()
                };
                fn_env.insert(fd.name.clone(), InferFnSig { params, ret })?;
            }
            Top::Def(d) => {
                if env.structs.contains_key(&d.name)
                    || env.unions.contains_key(&d.name)
                    || env.variants.contains_key(&d.name)
                    || fn_env.get(&d.name).is_some()
                    || global_defs.contains_key(&d.name)
                {
                    return Err(
                        Diag::new(format!("duplicate def '{}'", d.name)).with_span(d.span.clone())
                    );
                }
                let ty = if let Some(ann) = &d.ann {
                    infer_from_ty(&mut ctx, ann)
                } else {
                    ctx.fresh_var()
                };
                global_defs.insert(d.name.clone(), ty);
            }
            _ => {}
        }
    }

    let mut infer_fns = Vec::new();
    let mut infer_defs = Vec::new();
    for t in &filtered {
        match t {
            Top::Func(fd) => {
                let sig = fn_env.get_arity(&fd.name, fd.params.len()).ok_or_else(|| {
                    Diag::new("internal error: missing function signature")
                        .with_span(fd.span.clone())
                })?;
                let (param_tys, param_spans) = build_param_maps(fd, sig)?;
                apply_param_field_constraints(&env, fd, &param_tys, &param_spans, &mut ctx)?;
                let body = if fd.extern_ {
                    InferExpr {
                        expr: fd.body.clone(),
                        ty: sig.ret.clone(),
                        casts: vec![],
                        types: BTreeMap::new(),
                    }
                } else {
                    let vars = param_tys.clone();
                    let infer_body = infer_expr_type(
                        &env,
                        &field_env,
                        &fn_env,
                        &global_defs,
                        &def_base_names,
                        &mut ctx,
                        &vars,
                        Some(&fd.name),
                        &fd.body,
                    )?;
                    ctx.unify(&sig.ret, &infer_body.ty, &fd.body.span())?;
                    infer_body
                };
                infer_fns.push(InferFnState {
                    def: fd.clone(),
                    param_tys,
                    body,
                });
            }
            Top::Def(d) => {
                let def_ty = global_defs.get(&d.name).ok_or_else(|| {
                    Diag::new("internal error: missing def type").with_span(d.span.clone())
                })?;
                let vars: BTreeMap<String, InferTy> = BTreeMap::new();
                let mut infer_body = infer_expr_type(
                    &env,
                    &field_env,
                    &fn_env,
                    &global_defs,
                    &def_base_names,
                    &mut ctx,
                    &vars,
                    None,
                    &d.expr,
                )?;
                ctx.unify(def_ty, &infer_body.ty, &d.span)?;
                infer_body.ty = def_ty.clone();
                infer_defs.push(InferDefState {
                    def: d.clone(),
                    body: infer_body,
                });
            }
            _ => {}
        }
    }

    let mut keep_vars = BTreeSet::new();
    for inf in &infer_fns {
        for ty in inf.param_tys.values() {
            let resolved = ctx.resolve(ty);
            collect_infer_var_ids(&resolved, &mut keep_vars);
        }
    }
    check_numeric_constraints(&mut ctx, &keep_vars)?;
    check_int_constraints(&mut ctx, &keep_vars)?;
    let env = apply_field_inference(&env, &field_env, &ctx)?;
    let mut typed_fns = Vec::new();
    let mut typed_defs = Vec::new();

    for inf in infer_fns {
        let mut final_param_tys = BTreeMap::new();
        for (name, ty) in &inf.param_tys {
            let resolved = infer_to_ty_allow_generic(&ctx, ty);
            final_param_tys.insert(name.clone(), resolved);
        }
        let body = finalize_infer_expr_allow_generic(&ctx, inf.body).map_err(|mut d| {
            if d.span.is_none() {
                d = d.with_span(inf.def.body.span());
            }
            d
        })?;
        let mut mutated = BTreeSet::new();
        collect_mutated_vars(&body.expr, &mut mutated);
        typed_fns.push(TypedFn {
            def: inf.def,
            param_tys: final_param_tys,
            param_modes: BTreeMap::new(),
            generic_bounds: BTreeMap::new(),
            body,
            mutated,
        });
    }

    typed_fns = specialize_internal_typed_fns(typed_fns);

    for inf in infer_defs {
        let body = finalize_infer_expr(&ctx, inf.body).map_err(|mut diag| {
            if diag.span.is_none() {
                diag = diag.with_span(inf.def.expr.span());
            }
            diag
        })?;
        typed_defs.push(TypedDef { def: inf.def, body });
    }

    let mut program = TypecheckedProgram {
        env,
        typed_fns,
        typed_defs,
        fn_env: FnEnv::new(),
        auto_clones: BTreeSet::new(),
        warnings: Vec::new(),
    };
    let fn_env = infer_param_modes(&program);
    apply_param_modes(&mut program.typed_fns, &fn_env);
    let move_plan = analyze_moves(&program, &fn_env);
    program.fn_env = fn_env;
    program.auto_clones = move_plan.auto_clones;
    program.warnings = move_plan.warnings;
    for f in &mut program.typed_fns {
        let mut bounds = BTreeMap::new();
        collect_bounds_expr(
            &f.body.expr,
            &f.body.types,
            &program.auto_clones,
            &mut bounds,
        );
        f.generic_bounds = bounds;
    }

    enforce_export_signatures(&program)?;
    Ok(program)
}

fn enforce_export_signatures(program: &TypecheckedProgram) -> DslResult<()> {
    for f in &program.typed_fns {
        if !f.def.exported || f.def.rust_name == "main" {
            continue;
        }
        for (name, ty) in &f.param_tys {
            if ty_contains_generic(ty) {
                return Err(Diag::new(format!(
                    "export requires explicit types; parameter '{}' is not fully known",
                    name
                ))
                .with_span(f.def.span.clone()));
            }
        }
        if ty_contains_generic(&f.body.ty) {
            return Err(Diag::new(
                "export requires explicit types; return type is not fully known",
            )
            .with_span(f.def.span.clone()));
        }
    }
    Ok(())
}

fn ty_contains_generic(ty: &Ty) -> bool {
    match ty {
        Ty::Generic(_) => true,
        Ty::Vec(inner) | Ty::Set(inner) | Ty::Option(inner) => ty_contains_generic(inner),
        Ty::Result(ok, err) => ty_contains_generic(ok) || ty_contains_generic(err),
        Ty::Map(_, k, v) => ty_contains_generic(k) || ty_contains_generic(v),
        Ty::Union(items) => items.iter().any(ty_contains_generic),
        Ty::Named(_) | Ty::Unknown => false,
    }
}

fn specialize_internal_typed_fns(mut fns: Vec<TypedFn>) -> Vec<TypedFn> {
    let mut candidates: BTreeMap<String, TypedFn> = BTreeMap::new();
    for f in &fns {
        if f.def.exported || f.def.extern_ {
            continue;
        }
        let mut has_generics = false;
        for ty in f.param_tys.values() {
            if ty_contains_generic(ty) {
                has_generics = true;
                break;
            }
        }
        if !has_generics && ty_contains_generic(&f.body.ty) {
            has_generics = true;
        }
        if has_generics {
            candidates.insert(f.def.name.clone(), f.clone());
        }
    }
    if candidates.is_empty() {
        return fns;
    }

    let mut spec_name_by_key: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut to_remove: BTreeSet<String> = BTreeSet::new();

    let mut generated_names: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<TypedFn> = Vec::new();

    let mut idx = 0usize;
    while idx < fns.len() {
        {
            let f = &mut fns[idx];
            rewrite_calls_for_specialization(
                &mut f.body.expr,
                &f.body.types,
                &candidates,
                &mut spec_name_by_key,
                &mut generated_names,
                &mut pending,
                &mut to_remove,
            );
        }
        if !pending.is_empty() {
            fns.extend(pending.drain(..));
        }
        idx += 1;
    }

    fns.retain(|f| !to_remove.contains(&f.def.name));
    fns
}

fn rewrite_calls_for_specialization(
    expr: &mut Expr,
    types: &BTreeMap<SpanKey, Ty>,
    candidates: &BTreeMap<String, TypedFn>,
    spec_name_by_key: &mut BTreeMap<(String, String), String>,
    generated_names: &mut BTreeSet<String>,
    pending: &mut Vec<TypedFn>,
    to_remove: &mut BTreeSet<String>,
) {
    match expr {
        Expr::Call { op, args, span: _ } => {
            for a in args.iter_mut() {
                rewrite_calls_for_specialization(
                    a,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }

            let Some(template) = candidates.get(op) else {
                return;
            };
            if template.def.exported || template.def.extern_ {
                return;
            }
            let mut actuals = Vec::new();
            for a in args.iter() {
                let Some(ty) = types.get(&SpanKey::new(&a.span())).cloned() else {
                    return;
                };
                if ty_contains_generic(&ty) {
                    return;
                }
                actuals.push(ty);
            }

            let mut subst: BTreeMap<u32, Ty> = BTreeMap::new();
            for (idx, p) in template.def.params.iter().enumerate() {
                let Some(pat) = template.param_tys.get(&p.rust_name) else {
                    return;
                };
                if idx >= actuals.len() {
                    return;
                }
                if !match_ty_pattern(pat, &actuals[idx], &mut subst) {
                    return;
                }
            }
            if subst.is_empty() {
                return;
            }
            let mut parts = Vec::new();
            for (id, ty) in &subst {
                parts.push(format!("T{}={}", id, ty.rust()));
            }
            let key_str = parts.join(",");
            let entry_key = (template.def.name.clone(), key_str.clone());
            let spec_name = spec_name_by_key.entry(entry_key).or_insert_with(|| {
                let base = template.def.rust_name.clone();
                let hash = fnv1a_64(&key_str);
                format!("{}_spec_{:x}", base, hash)
            });

            to_remove.insert(template.def.name.clone());
            op.clone_from(spec_name);

            if generated_names.contains(spec_name) {
                return;
            }
            let spec_fn = substitute_typed_fn(template.clone(), spec_name.clone(), &subst);
            generated_names.insert(spec_name.clone());
            pending.push(spec_fn);
        }
        Expr::Ascribe { expr, .. } => rewrite_calls_for_specialization(
            expr,
            types,
            candidates,
            spec_name_by_key,
            generated_names,
            pending,
            to_remove,
        ),
        Expr::Cast { expr, .. } => rewrite_calls_for_specialization(
            expr,
            types,
            candidates,
            spec_name_by_key,
            generated_names,
            pending,
            to_remove,
        ),
        Expr::Let { bindings, body, .. } => {
            for b in bindings {
                rewrite_calls_for_specialization(
                    &mut b.expr,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
            rewrite_calls_for_specialization(
                body,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
        }
        Expr::Lambda { body, .. } => rewrite_calls_for_specialization(
            body,
            types,
            candidates,
            spec_name_by_key,
            generated_names,
            pending,
            to_remove,
        ),
        Expr::CallDyn { func, args, .. } => {
            rewrite_calls_for_specialization(
                func,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
            for a in args.iter_mut() {
                rewrite_calls_for_specialization(
                    a,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
        }
        Expr::MethodCall { base, args, .. } => {
            rewrite_calls_for_specialization(
                base,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
            for a in args.iter_mut() {
                rewrite_calls_for_specialization(
                    a,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
        }
        Expr::Do { exprs, .. } => {
            for e in exprs.iter_mut() {
                rewrite_calls_for_specialization(
                    e,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            rewrite_calls_for_specialization(
                cond,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
            rewrite_calls_for_specialization(
                then_br,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
            if let Some(else_br) = else_br {
                rewrite_calls_for_specialization(
                    else_br,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
        }
        Expr::Loop { body, .. } | Expr::While { body, .. } => rewrite_calls_for_specialization(
            body,
            types,
            candidates,
            spec_name_by_key,
            generated_names,
            pending,
            to_remove,
        ),
        Expr::For { iter, body, .. } => {
            if let crate::ast::Iterable::Expr(ex) = iter {
                rewrite_calls_for_specialization(
                    ex,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
            rewrite_calls_for_specialization(
                body,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            rewrite_calls_for_specialization(
                scrutinee,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
            for arm in arms {
                rewrite_calls_for_specialization(
                    &mut arm.body,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
        }
        Expr::VecLit { elems, .. } | Expr::SetLit { elems, .. } => {
            for e in elems.iter_mut() {
                rewrite_calls_for_specialization(
                    e,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                rewrite_calls_for_specialization(
                    k,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
                rewrite_calls_for_specialization(
                    v,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
        }
        Expr::Pair { key, val, .. } => {
            rewrite_calls_for_specialization(
                key,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
            rewrite_calls_for_specialization(
                val,
                types,
                candidates,
                spec_name_by_key,
                generated_names,
                pending,
                to_remove,
            );
        }
        Expr::Break { value, .. } => {
            if let Some(v) = value {
                rewrite_calls_for_specialization(
                    v,
                    types,
                    candidates,
                    spec_name_by_key,
                    generated_names,
                    pending,
                    to_remove,
                );
            }
        }
        Expr::Set { expr, .. } => rewrite_calls_for_specialization(
            expr,
            types,
            candidates,
            spec_name_by_key,
            generated_names,
            pending,
            to_remove,
        ),
        Expr::Field { base, .. } => rewrite_calls_for_specialization(
            base,
            types,
            candidates,
            spec_name_by_key,
            generated_names,
            pending,
            to_remove,
        ),
        Expr::Var(..)
        | Expr::Int(..)
        | Expr::Float(..)
        | Expr::Str(..)
        | Expr::Bool(..)
        | Expr::Unit(..)
        | Expr::Keyword(..)
        | Expr::Continue { .. } => {}
    }
}

fn substitute_typed_fn(mut f: TypedFn, new_name: String, subst: &BTreeMap<u32, Ty>) -> TypedFn {
    f.def.name = new_name.clone();
    f.def.rust_name = new_name.clone();
    f.def.exported = false;
    f.def.specialize = false;
    for v in f.param_tys.values_mut() {
        *v = substitute_ty(v, subst);
    }
    f.body.ty = substitute_ty(&f.body.ty, subst);
    for v in f.body.types.values_mut() {
        *v = substitute_ty(v, subst);
    }
    for ch in f.body.casts.iter_mut() {
        ch.target = substitute_ty(&ch.target, subst);
    }
    f.generic_bounds.clear();
    f
}

fn substitute_ty(ty: &Ty, subst: &BTreeMap<u32, Ty>) -> Ty {
    match ty {
        Ty::Generic(id) => subst.get(id).cloned().unwrap_or_else(|| ty.clone()),
        Ty::Vec(inner) => Ty::Vec(Box::new(substitute_ty(inner, subst))),
        Ty::Set(inner) => Ty::Set(Box::new(substitute_ty(inner, subst))),
        Ty::Option(inner) => Ty::Option(Box::new(substitute_ty(inner, subst))),
        Ty::Result(ok, err) => Ty::Result(
            Box::new(substitute_ty(ok, subst)),
            Box::new(substitute_ty(err, subst)),
        ),
        Ty::Map(k, kt, vt) => Ty::Map(
            k.clone(),
            Box::new(substitute_ty(kt, subst)),
            Box::new(substitute_ty(vt, subst)),
        ),
        Ty::Union(items) => Ty::Union(items.iter().map(|t| substitute_ty(t, subst)).collect()),
        Ty::Named(_) | Ty::Unknown => ty.clone(),
    }
}

fn match_ty_pattern(pat: &Ty, actual: &Ty, subst: &mut BTreeMap<u32, Ty>) -> bool {
    match (pat, actual) {
        (Ty::Generic(id), _) => match subst.get(id) {
            Some(existing) => existing == actual,
            None => {
                subst.insert(*id, actual.clone());
                true
            }
        },
        (Ty::Named(a), Ty::Named(b)) => a == b,
        (Ty::Vec(a), Ty::Vec(b)) | (Ty::Set(a), Ty::Set(b)) | (Ty::Option(a), Ty::Option(b)) => {
            match_ty_pattern(a, b, subst)
        }
        (Ty::Result(a1, a2), Ty::Result(b1, b2)) => {
            match_ty_pattern(a1, b1, subst) && match_ty_pattern(a2, b2, subst)
        }
        (Ty::Map(ka, a1, a2), Ty::Map(kb, b1, b2)) => {
            ka == kb && match_ty_pattern(a1, b1, subst) && match_ty_pattern(a2, b2, subst)
        }
        _ => false,
    }
}

fn fnv1a_64(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn collect_def_base_names(tops: &[Top]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for t in tops {
        if let Top::Def(d) = t {
            out.insert(def_base_name(&d.name));
        }
    }
    out
}

fn def_base_name(name: &str) -> String {
    let base = name.rsplit_once('/').map(|(_, tail)| tail).unwrap_or(name);
    rust_value_name(base)
}

fn rust_value_name(name: &str) -> String {
    let base = name.rsplit_once('/').map(|(_, tail)| tail).unwrap_or(name);
    let mut out = String::new();
    for c in base.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => out.push(c),
            '-' | '_' => out.push('_'),
            '?' => out.push_str("_q"),
            '!' => out.push_str("_bang"),
            '*' => out.push_str("_star"),
            '+' => out.push_str("_plus"),
            '=' => out.push_str("_eq"),
            '<' => out.push_str("_lt"),
            '>' => out.push_str("_gt"),
            '$' => out.push_str("_dollar"),
            '&' => out.push_str("_and"),
            '|' => out.push_str("_or"),
            _ => out.push('_'),
        }
    }
    if out.is_empty() {
        out.push('v');
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

#[derive(Debug, Clone)]
struct MovePlan {
    auto_clones: BTreeSet<SpanKey>,
    warnings: Vec<Diag>,
}

fn infer_param_modes(program: &TypecheckedProgram) -> FnEnv {
    let builtin_modes = builtin_param_modes();
    let mut fn_modes: BTreeMap<String, Vec<ParamMode>> = BTreeMap::new();
    for f in &program.typed_fns {
        fn_modes.insert(
            f.def.name.clone(),
            vec![ParamMode::ByVal; f.def.params.len()],
        );
    }

    for _ in 0..16 {
        let mut changed = false;
        for f in &program.typed_fns {
            if f.def.extern_ || f.def.params.is_empty() {
                continue;
            }
            let consumed =
                param_consumption_in_expr(&f.body.expr, &f.param_tys, &builtin_modes, &fn_modes);
            let mut next_modes = Vec::new();
            for p in &f.def.params {
                let ty = f
                    .param_tys
                    .get(&p.rust_name)
                    .cloned()
                    .unwrap_or(Ty::Unknown);
                let mode = if is_copy_type(&ty) {
                    ParamMode::ByVal
                } else if consumed.contains(&p.rust_name) {
                    ParamMode::ByVal
                } else {
                    ParamMode::ByRef
                };
                next_modes.push(mode);
            }
            if fn_modes.get(&f.def.name) != Some(&next_modes) {
                fn_modes.insert(f.def.name.clone(), next_modes);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut env = FnEnv::new();
    for (name, modes) in &builtin_modes {
        let params = vec![Ty::Unknown; modes.len()];
        let sig = FnSig {
            params,
            param_modes: modes.clone(),
            ret: Ty::Unknown,
        };
        let _ = env.insert(name.clone(), sig);
    }
    for f in &program.typed_fns {
        let mut params = Vec::new();
        for p in &f.def.params {
            params.push(
                f.param_tys
                    .get(&p.rust_name)
                    .cloned()
                    .unwrap_or(Ty::Unknown),
            );
        }
        let modes = fn_modes
            .get(&f.def.name)
            .cloned()
            .unwrap_or_else(|| vec![ParamMode::ByVal; f.def.params.len()]);
        let sig = FnSig {
            params,
            param_modes: modes,
            ret: f.body.ty.clone(),
        };
        let _ = env.insert(f.def.name.clone(), sig);
    }
    env
}

fn apply_param_modes(fns: &mut [TypedFn], fn_env: &FnEnv) {
    for f in fns {
        let sig = match fn_env.get_arity(&f.def.name, f.def.params.len()) {
            Some(sig) => sig,
            None => continue,
        };
        let mut modes = BTreeMap::new();
        for (idx, p) in f.def.params.iter().enumerate() {
            if let Some(mode) = sig.param_modes.get(idx) {
                modes.insert(p.rust_name.clone(), *mode);
            }
        }
        f.param_modes = modes;
    }
}

fn analyze_moves(program: &TypecheckedProgram, fn_env: &FnEnv) -> MovePlan {
    let mut plan = MovePlan {
        auto_clones: BTreeSet::new(),
        warnings: Vec::new(),
    };
    for f in &program.typed_fns {
        if f.def.extern_ {
            continue;
        }
        let mut use_counts = BTreeMap::new();
        let mut count_types = f.param_tys.clone();
        let mut count_locals = BTreeSet::new();
        count_consumed_uses(
            &f.body.expr,
            &f.body.types,
            fn_env,
            &mut count_types,
            &mut count_locals,
            &mut use_counts,
        );
        let mut moved = BTreeSet::new();
        let mut locals = BTreeSet::new();
        let mut types = f.param_tys.clone();
        analyze_moves_expr(
            &f.body.expr,
            &f.body.types,
            fn_env,
            &mut types,
            &mut locals,
            &mut moved,
            &mut plan,
            &mut use_counts,
            false,
        );
    }
    plan
}

fn analyze_moves_expr(
    expr: &Expr,
    types_map: &BTreeMap<SpanKey, Ty>,
    fn_env: &FnEnv,
    types: &mut BTreeMap<String, Ty>,
    locals: &mut BTreeSet<String>,
    moved: &mut BTreeSet<String>,
    plan: &mut MovePlan,
    use_counts: &mut BTreeMap<String, usize>,
    in_loop: bool,
) {
    match expr {
        Expr::Var(name, sp) => {
            consume_var(name, sp, types, locals, moved, plan, use_counts, in_loop);
        }
        Expr::Ascribe { expr, .. } => {
            analyze_moves_expr(
                expr, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
        }
        Expr::Cast { expr, .. } => {
            analyze_moves_expr(
                expr, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
        }
        Expr::Let { bindings, body, .. } => {
            for b in bindings {
                analyze_moves_expr(
                    &b.expr, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
                let ty = types_map
                    .get(&SpanKey::new(&b.expr.span()))
                    .cloned()
                    .unwrap_or(Ty::Unknown);
                types.insert(b.rust_name.clone(), ty);
                locals.insert(b.rust_name.clone());
            }
            analyze_moves_expr(
                body, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
        }
        Expr::Lambda { params, body, .. } => {
            let mut nested_locals = locals.clone();
            let mut nested_types = types.clone();
            for p in params {
                nested_locals.insert(p.rust_name.clone());
                nested_types.insert(p.rust_name.clone(), Ty::Unknown);
            }
            let mut nested_moved = BTreeSet::new();
            analyze_moves_expr(
                body,
                types_map,
                fn_env,
                &mut nested_types,
                &mut nested_locals,
                &mut nested_moved,
                plan,
                use_counts,
                in_loop,
            );
        }
        Expr::Call { op, args, .. } => {
            let modes = fn_env
                .get_arity(op, args.len())
                .map(|s| s.param_modes.clone());
            for (idx, arg) in args.iter().enumerate() {
                let mode = modes
                    .as_ref()
                    .and_then(|m| m.get(idx))
                    .copied()
                    .unwrap_or(ParamMode::ByVal);
                if matches!(mode, ParamMode::ByRef | ParamMode::ByRefNoAmp) {
                    analyze_moves_borrow(
                        arg, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                    );
                } else {
                    analyze_moves_expr(
                        arg, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                    );
                }
            }
        }
        Expr::CallDyn { args, .. } => {
            for arg in args {
                analyze_moves_expr(
                    arg, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
            }
        }
        Expr::MethodCall { base, args, .. } => {
            analyze_moves_expr(
                base, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
            for arg in args {
                analyze_moves_expr(
                    arg, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
            }
        }
        Expr::Field { base, .. } => {
            analyze_moves_borrow(
                base, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
        }
        Expr::Do { exprs, .. } => {
            for e in exprs {
                analyze_moves_expr(
                    e, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            analyze_moves_expr(
                cond, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
            let mut moved_then = moved.clone();
            analyze_moves_expr(
                then_br,
                types_map,
                fn_env,
                types,
                locals,
                &mut moved_then,
                plan,
                use_counts,
                in_loop,
            );
            if let Some(else_br) = else_br {
                let mut moved_else = moved.clone();
                analyze_moves_expr(
                    else_br,
                    types_map,
                    fn_env,
                    types,
                    locals,
                    &mut moved_else,
                    plan,
                    use_counts,
                    in_loop,
                );
                moved.extend(moved_then.into_iter());
                moved.extend(moved_else.into_iter());
            } else {
                moved.extend(moved_then.into_iter());
            }
        }
        Expr::Loop { body, .. } | Expr::While { body, .. } => {
            analyze_moves_expr(
                body, types_map, fn_env, types, locals, moved, plan, use_counts, true,
            );
        }
        Expr::For { body, .. } => {
            analyze_moves_expr(
                body, types_map, fn_env, types, locals, moved, plan, use_counts, true,
            );
        }
        Expr::Set { expr, .. } => {
            analyze_moves_expr(
                expr, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            analyze_moves_expr(
                scrutinee, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
            for arm in arms {
                analyze_moves_expr(
                    &arm.body, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
            }
        }
        Expr::VecLit { elems, .. } | Expr::SetLit { elems, .. } => {
            for e in elems {
                analyze_moves_expr(
                    e, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                analyze_moves_expr(
                    k, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
                analyze_moves_expr(
                    v, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
            }
        }
        Expr::Pair { key, val, .. } => {
            analyze_moves_expr(
                key, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
            analyze_moves_expr(
                val, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
            );
        }
        Expr::Break { value, .. } => {
            if let Some(v) = value {
                analyze_moves_expr(
                    v, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
                );
            }
        }
        Expr::Int(..)
        | Expr::Float(..)
        | Expr::Str(..)
        | Expr::Bool(..)
        | Expr::Unit(..)
        | Expr::Keyword(..)
        | Expr::Continue { .. } => {}
    }
}

fn analyze_moves_borrow(
    expr: &Expr,
    types_map: &BTreeMap<SpanKey, Ty>,
    fn_env: &FnEnv,
    types: &mut BTreeMap<String, Ty>,
    locals: &mut BTreeSet<String>,
    moved: &mut BTreeSet<String>,
    plan: &mut MovePlan,
    use_counts: &mut BTreeMap<String, usize>,
    in_loop: bool,
) {
    match expr {
        Expr::Var(name, sp) => {
            if should_consume_var(name, types, moved) {
                let key = SpanKey::new(sp);
                if plan.auto_clones.insert(key) {
                    plan.warnings.push(
                        Diag::new(format!(
                            "auto-cloned '{}' to avoid move; consider darcy.core/clone",
                            name
                        ))
                        .with_span(sp.clone()),
                    );
                }
            }
        }
        _ => analyze_moves_expr(
            expr, types_map, fn_env, types, locals, moved, plan, use_counts, in_loop,
        ),
    }
}

fn consume_var(
    name: &str,
    sp: &Span,
    types: &BTreeMap<String, Ty>,
    locals: &BTreeSet<String>,
    moved: &mut BTreeSet<String>,
    plan: &mut MovePlan,
    use_counts: &mut BTreeMap<String, usize>,
    in_loop: bool,
) {
    if in_loop && !locals.contains(name) {
        if let Some(ty) = types.get(name) {
            if !is_copy_type(ty) {
                let key = SpanKey::new(sp);
                plan.auto_clones.insert(key);
                dec_use_count(use_counts, name);
                return;
            }
        }
    }
    let remaining = use_counts.get(name).copied().unwrap_or(0);
    if remaining > 1 {
        let key = SpanKey::new(sp);
        plan.auto_clones.insert(key);
        dec_use_count(use_counts, name);
        return;
    }
    if should_consume_var(name, types, moved) {
        let key = SpanKey::new(sp);
        plan.auto_clones.insert(key);
    } else if types.contains_key(name) && !is_copy_type(types.get(name).unwrap()) {
        moved.insert(name.to_string());
    }
    dec_use_count(use_counts, name);
}

fn should_consume_var(name: &str, types: &BTreeMap<String, Ty>, moved: &BTreeSet<String>) -> bool {
    if let Some(ty) = types.get(name) {
        if is_copy_type(ty) {
            return false;
        }
        return moved.contains(name);
    }
    false
}

fn dec_use_count(counts: &mut BTreeMap<String, usize>, name: &str) {
    if let Some(v) = counts.get_mut(name) {
        if *v > 0 {
            *v -= 1;
        }
    }
}

fn count_consumed_uses(
    expr: &Expr,
    types_map: &BTreeMap<SpanKey, Ty>,
    fn_env: &FnEnv,
    types: &mut BTreeMap<String, Ty>,
    locals: &mut BTreeSet<String>,
    counts: &mut BTreeMap<String, usize>,
) {
    match expr {
        Expr::Var(name, _) => {
            if let Some(ty) = types.get(name) {
                if !is_copy_type(ty) {
                    *counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }
        Expr::Ascribe { expr, .. } | Expr::Cast { expr, .. } => {
            count_consumed_uses(expr, types_map, fn_env, types, locals, counts);
        }
        Expr::Let { bindings, body, .. } => {
            for b in bindings {
                count_consumed_uses(&b.expr, types_map, fn_env, types, locals, counts);
                let ty = types_map
                    .get(&SpanKey::new(&b.expr.span()))
                    .cloned()
                    .unwrap_or(Ty::Unknown);
                types.insert(b.rust_name.clone(), ty);
                locals.insert(b.rust_name.clone());
            }
            count_consumed_uses(body, types_map, fn_env, types, locals, counts);
        }
        Expr::Lambda { params, body, .. } => {
            let mut nested_locals = locals.clone();
            let mut nested_types = types.clone();
            for p in params {
                nested_locals.insert(p.rust_name.clone());
                nested_types.insert(p.rust_name.clone(), Ty::Unknown);
            }
            count_consumed_uses(
                body,
                types_map,
                fn_env,
                &mut nested_types,
                &mut nested_locals,
                counts,
            );
        }
        Expr::Call { op, args, .. } => {
            let modes = fn_env
                .get_arity(op, args.len())
                .map(|s| s.param_modes.clone());
            for (idx, arg) in args.iter().enumerate() {
                let mode = modes
                    .as_ref()
                    .and_then(|m| m.get(idx))
                    .copied()
                    .unwrap_or(ParamMode::ByVal);
                if matches!(mode, ParamMode::ByRef | ParamMode::ByRefNoAmp) {
                    // Borrowed args do not consume.
                    continue;
                }
                count_consumed_uses(arg, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::CallDyn { args, .. } => {
            for arg in args {
                count_consumed_uses(arg, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::MethodCall { base, args, .. } => {
            count_consumed_uses(base, types_map, fn_env, types, locals, counts);
            for arg in args {
                count_consumed_uses(arg, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::Field { base, .. } => {
            count_consumed_uses(base, types_map, fn_env, types, locals, counts);
        }
        Expr::Do { exprs, .. } => {
            for e in exprs {
                count_consumed_uses(e, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            count_consumed_uses(cond, types_map, fn_env, types, locals, counts);
            count_consumed_uses(then_br, types_map, fn_env, types, locals, counts);
            if let Some(else_br) = else_br {
                count_consumed_uses(else_br, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::Loop { body, .. } | Expr::While { body, .. } | Expr::For { body, .. } => {
            count_consumed_uses(body, types_map, fn_env, types, locals, counts);
        }
        Expr::Set { expr, .. } => {
            count_consumed_uses(expr, types_map, fn_env, types, locals, counts);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            count_consumed_uses(scrutinee, types_map, fn_env, types, locals, counts);
            for arm in arms {
                count_consumed_uses(&arm.body, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::VecLit { elems, .. } | Expr::SetLit { elems, .. } => {
            for e in elems {
                count_consumed_uses(e, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                count_consumed_uses(k, types_map, fn_env, types, locals, counts);
                count_consumed_uses(v, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::Pair { key, val, .. } => {
            count_consumed_uses(key, types_map, fn_env, types, locals, counts);
            count_consumed_uses(val, types_map, fn_env, types, locals, counts);
        }
        Expr::Break { value, .. } => {
            if let Some(v) = value {
                count_consumed_uses(v, types_map, fn_env, types, locals, counts);
            }
        }
        Expr::Int(..)
        | Expr::Float(..)
        | Expr::Str(..)
        | Expr::Bool(..)
        | Expr::Unit(..)
        | Expr::Keyword(..)
        | Expr::Continue { .. } => {}
    }
}

fn param_consumption_in_expr(
    expr: &Expr,
    param_tys: &BTreeMap<String, Ty>,
    builtin_modes: &BTreeMap<String, Vec<ParamMode>>,
    fn_modes: &BTreeMap<String, Vec<ParamMode>>,
) -> BTreeSet<String> {
    let mut locals = BTreeSet::new();
    let mut out = BTreeSet::new();
    param_consumption_visit(
        expr,
        param_tys,
        builtin_modes,
        fn_modes,
        &mut locals,
        &mut out,
    );
    out
}

fn param_consumption_visit(
    expr: &Expr,
    param_tys: &BTreeMap<String, Ty>,
    builtin_modes: &BTreeMap<String, Vec<ParamMode>>,
    fn_modes: &BTreeMap<String, Vec<ParamMode>>,
    locals: &mut BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    match expr {
        Expr::Var(name, _) => {
            if locals.contains(name) {
                return;
            }
            if let Some(ty) = param_tys.get(name) {
                if !is_copy_type(ty) {
                    out.insert(name.clone());
                }
            }
        }
        Expr::Ascribe { expr, .. } => {
            param_consumption_visit(expr, param_tys, builtin_modes, fn_modes, locals, out);
        }
        Expr::Cast { expr, .. } => {
            param_consumption_visit(expr, param_tys, builtin_modes, fn_modes, locals, out);
        }
        Expr::Let { bindings, body, .. } => {
            for b in bindings {
                param_consumption_visit(&b.expr, param_tys, builtin_modes, fn_modes, locals, out);
                locals.insert(b.rust_name.clone());
            }
            param_consumption_visit(body, param_tys, builtin_modes, fn_modes, locals, out);
        }
        Expr::Lambda { params, body, .. } => {
            let mut nested = locals.clone();
            for p in params {
                nested.insert(p.rust_name.clone());
            }
            param_consumption_visit(body, param_tys, builtin_modes, fn_modes, &mut nested, out);
        }
        Expr::Call { op, args, .. } => {
            let modes = fn_modes.get(op).or_else(|| builtin_modes.get(op)).cloned();
            for (idx, arg) in args.iter().enumerate() {
                let mode = modes
                    .as_ref()
                    .and_then(|m| m.get(idx))
                    .copied()
                    .unwrap_or(ParamMode::ByVal);
                if matches!(mode, ParamMode::ByRef | ParamMode::ByRefNoAmp) {
                    param_consumption_borrow(arg, param_tys, builtin_modes, fn_modes, locals, out);
                } else {
                    param_consumption_visit(arg, param_tys, builtin_modes, fn_modes, locals, out);
                }
            }
        }
        Expr::CallDyn { args, .. } => {
            for arg in args {
                param_consumption_visit(arg, param_tys, builtin_modes, fn_modes, locals, out);
            }
        }
        Expr::MethodCall { base, args, .. } => {
            param_consumption_visit(base, param_tys, builtin_modes, fn_modes, locals, out);
            for arg in args {
                param_consumption_visit(arg, param_tys, builtin_modes, fn_modes, locals, out);
            }
        }
        Expr::Field { base, .. } => {
            param_consumption_borrow(base, param_tys, builtin_modes, fn_modes, locals, out);
        }
        Expr::Do { exprs, .. } => {
            for e in exprs {
                param_consumption_visit(e, param_tys, builtin_modes, fn_modes, locals, out);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            param_consumption_visit(cond, param_tys, builtin_modes, fn_modes, locals, out);
            param_consumption_visit(then_br, param_tys, builtin_modes, fn_modes, locals, out);
            if let Some(else_br) = else_br {
                param_consumption_visit(else_br, param_tys, builtin_modes, fn_modes, locals, out);
            }
        }
        Expr::Loop { body, .. } | Expr::While { body, .. } => {
            param_consumption_visit(body, param_tys, builtin_modes, fn_modes, locals, out);
        }
        Expr::For { body, .. } => {
            param_consumption_visit(body, param_tys, builtin_modes, fn_modes, locals, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            param_consumption_visit(scrutinee, param_tys, builtin_modes, fn_modes, locals, out);
            for arm in arms {
                param_consumption_visit(&arm.body, param_tys, builtin_modes, fn_modes, locals, out);
            }
        }
        Expr::VecLit { elems, .. } | Expr::SetLit { elems, .. } => {
            for e in elems {
                param_consumption_visit(e, param_tys, builtin_modes, fn_modes, locals, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                param_consumption_visit(k, param_tys, builtin_modes, fn_modes, locals, out);
                param_consumption_visit(v, param_tys, builtin_modes, fn_modes, locals, out);
            }
        }
        Expr::Pair { key, val, .. } => {
            param_consumption_visit(key, param_tys, builtin_modes, fn_modes, locals, out);
            param_consumption_visit(val, param_tys, builtin_modes, fn_modes, locals, out);
        }
        Expr::Break { value, .. } => {
            if let Some(v) = value {
                param_consumption_visit(v, param_tys, builtin_modes, fn_modes, locals, out);
            }
        }
        Expr::Set { expr, .. } => {
            param_consumption_visit(expr, param_tys, builtin_modes, fn_modes, locals, out);
        }
        Expr::Int(..)
        | Expr::Float(..)
        | Expr::Str(..)
        | Expr::Bool(..)
        | Expr::Unit(..)
        | Expr::Keyword(..)
        | Expr::Continue { .. } => {}
    }
}

fn param_consumption_borrow(
    expr: &Expr,
    param_tys: &BTreeMap<String, Ty>,
    builtin_modes: &BTreeMap<String, Vec<ParamMode>>,
    fn_modes: &BTreeMap<String, Vec<ParamMode>>,
    locals: &mut BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    match expr {
        Expr::Var(..) => {}
        _ => param_consumption_visit(expr, param_tys, builtin_modes, fn_modes, locals, out),
    }
}

fn builtin_param_modes() -> BTreeMap<String, Vec<ParamMode>> {
    let mut out = BTreeMap::new();
    out.insert("darcy.vec/len".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.vec/is-empty".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.vec/range".to_string(), vec![ParamMode::ByVal]);
    out.insert(
        "darcy.vec/get".to_string(),
        vec![ParamMode::ByRef, ParamMode::ByVal],
    );
    out.insert("darcy.string/len".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.string/is-empty".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.string/trim".to_string(), vec![ParamMode::ByRef]);
    out.insert(
        "darcy.string/split".to_string(),
        vec![ParamMode::ByRef, ParamMode::ByRef],
    );
    out.insert(
        "darcy.string/join".to_string(),
        vec![ParamMode::ByRef, ParamMode::ByRef],
    );
    out.insert("darcy.hash-map/len".to_string(), vec![ParamMode::ByRef]);
    out.insert(
        "darcy.hash-map/is-empty".to_string(),
        vec![ParamMode::ByRef],
    );
    out.insert(
        "darcy.hash-map/get".to_string(),
        vec![ParamMode::ByRef, ParamMode::ByVal],
    );
    out.insert(
        "darcy.hash-map/contains".to_string(),
        vec![ParamMode::ByRef, ParamMode::ByVal],
    );
    out.insert("darcy.btree-map/len".to_string(), vec![ParamMode::ByRef]);
    out.insert(
        "darcy.btree-map/is-empty".to_string(),
        vec![ParamMode::ByRef],
    );
    out.insert(
        "darcy.btree-map/get".to_string(),
        vec![ParamMode::ByRef, ParamMode::ByVal],
    );
    out.insert(
        "darcy.btree-map/contains".to_string(),
        vec![ParamMode::ByRef, ParamMode::ByVal],
    );
    out.insert("darcy.core/clone".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.vec/len".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.vec/is-empty".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.core/clone".to_string(), vec![ParamMode::ByRefNoAmp]);
    out.insert("darcy.io/dbg".to_string(), vec![ParamMode::ByRefNoAmp]);
    out.insert("darcy.string/len".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.string/is-empty".to_string(), vec![ParamMode::ByRef]);
    out.insert("darcy.string/trim".to_string(), vec![ParamMode::ByRef]);
    out.insert(
        "darcy.string/split".to_string(),
        vec![ParamMode::ByRef, ParamMode::ByRef],
    );
    out.insert(
        "darcy.string/join".to_string(),
        vec![ParamMode::ByVal, ParamMode::ByRef],
    );
    out.insert("darcy.fmt/pretty".to_string(), vec![ParamMode::ByRefNoAmp]);
    out
}

fn is_copy_type(ty: &Ty) -> bool {
    match ty {
        Ty::Named(n) => matches!(
            n.as_str(),
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "bool"
        ),
        Ty::Option(inner) | Ty::Result(inner, _) => is_copy_type(inner),
        Ty::Union(items) => items.iter().all(is_copy_type),
        _ => false,
    }
}

pub(crate) fn expand_inline_tops(tops: &[Top]) -> DslResult<Vec<Top>> {
    let mut inline_defs: BTreeMap<String, crate::ast::InlineDef> = BTreeMap::new();
    let mut filtered = Vec::new();
    for t in tops {
        match t {
            Top::Inline(inl) => {
                if inline_defs.contains_key(&inl.name) {
                    return Err(Diag::new(format!("duplicate inline '{}'", inl.name))
                        .with_span(inl.span.clone()));
                }
                inline_defs.insert(inl.name.clone(), inl.clone());
            }
            _ => filtered.push(t.clone()),
        }
    }
    if inline_defs.is_empty() {
        return Ok(filtered);
    }
    let mut out = Vec::new();
    for t in filtered {
        match t {
            Top::Func(mut fd) => {
                fd.body = expand_inline_calls(&fd.body, &inline_defs)?;
                out.push(Top::Func(fd));
            }
            Top::Def(mut d) => {
                d.expr = expand_inline_calls(&d.expr, &inline_defs)?;
                out.push(Top::Def(d));
            }
            _ => out.push(t),
        }
    }
    Ok(out)
}

fn expand_inline_calls(
    expr: &Expr,
    inline_defs: &BTreeMap<String, crate::ast::InlineDef>,
) -> DslResult<Expr> {
    match expr {
        Expr::Call { op, args, span } => {
            if let Some(inl) = inline_defs.get(op) {
                if inl.params.len() != args.len() {
                    return Err(Diag::new(format!(
                        "inline '{}' expects {} arguments",
                        inl.name,
                        inl.params.len()
                    ))
                    .with_span(span.clone()));
                }
                let mut map = BTreeMap::new();
                for (param, arg) in inl.params.iter().zip(args.iter()) {
                    map.insert(param.rust_name.clone(), arg.clone());
                }
                let expanded = inline_subst_local(&inl.body, &map);
                return expand_inline_calls(&expanded, inline_defs);
            }
            let mut out_args = Vec::new();
            for a in args {
                out_args.push(expand_inline_calls(a, inline_defs)?);
            }
            Ok(Expr::Call {
                op: op.clone(),
                args: out_args,
                span: span.clone(),
            })
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => Ok(Expr::If {
            cond: Box::new(expand_inline_calls(cond, inline_defs)?),
            then_br: Box::new(expand_inline_calls(then_br, inline_defs)?),
            else_br: match else_br {
                Some(b) => Some(Box::new(expand_inline_calls(b, inline_defs)?)),
                None => None,
            },
            span: span.clone(),
        }),
        Expr::Let {
            bindings,
            body,
            span,
        } => {
            let mut out = Vec::new();
            for b in bindings {
                out.push(crate::ast::LetBinding {
                    name: b.name.clone(),
                    rust_name: b.rust_name.clone(),
                    ann: b.ann.clone(),
                    expr: expand_inline_calls(&b.expr, inline_defs)?,
                    span: b.span.clone(),
                });
            }
            Ok(Expr::Let {
                bindings: out,
                body: Box::new(expand_inline_calls(body, inline_defs)?),
                span: span.clone(),
            })
        }
        Expr::Lambda { params, body, span } => Ok(Expr::Lambda {
            params: params.clone(),
            body: Box::new(expand_inline_calls(body, inline_defs)?),
            span: span.clone(),
        }),
        Expr::CallDyn { func, args, span } => {
            let func = Box::new(expand_inline_calls(func, inline_defs)?);
            let mut out_args = Vec::new();
            for a in args {
                out_args.push(expand_inline_calls(a, inline_defs)?);
            }
            Ok(Expr::CallDyn {
                func,
                args: out_args,
                span: span.clone(),
            })
        }
        Expr::MethodCall {
            base,
            method,
            args,
            span,
        } => {
            let base = Box::new(expand_inline_calls(base, inline_defs)?);
            let mut out_args = Vec::new();
            for a in args {
                out_args.push(expand_inline_calls(a, inline_defs)?);
            }
            Ok(Expr::MethodCall {
                base,
                method: method.clone(),
                args: out_args,
                span: span.clone(),
            })
        }
        Expr::Do { exprs, span } => {
            let mut out = Vec::new();
            for ex in exprs {
                out.push(expand_inline_calls(ex, inline_defs)?);
            }
            Ok(Expr::Do {
                exprs: out,
                span: span.clone(),
            })
        }
        Expr::Loop { body, span } => Ok(Expr::Loop {
            body: Box::new(expand_inline_calls(body, inline_defs)?),
            span: span.clone(),
        }),
        Expr::While { cond, body, span } => Ok(Expr::While {
            cond: Box::new(expand_inline_calls(cond, inline_defs)?),
            body: Box::new(expand_inline_calls(body, inline_defs)?),
            span: span.clone(),
        }),
        Expr::For {
            var,
            iter,
            body,
            span,
        } => Ok(Expr::For {
            var: var.clone(),
            iter: inline_subst_iterable_local(iter, inline_defs)?,
            body: Box::new(expand_inline_calls(body, inline_defs)?),
            span: span.clone(),
        }),
        Expr::Set { name, expr, span } => Ok(Expr::Set {
            name: name.clone(),
            expr: Box::new(expand_inline_calls(expr, inline_defs)?),
            span: span.clone(),
        }),
        Expr::Break { value, span } => Ok(Expr::Break {
            value: match value {
                Some(v) => Some(Box::new(expand_inline_calls(v, inline_defs)?)),
                None => None,
            },
            span: span.clone(),
        }),
        Expr::Pair { key, val, span } => Ok(Expr::Pair {
            key: Box::new(expand_inline_calls(key, inline_defs)?),
            val: Box::new(expand_inline_calls(val, inline_defs)?),
            span: span.clone(),
        }),
        Expr::Field { base, field, span } => Ok(Expr::Field {
            base: Box::new(expand_inline_calls(base, inline_defs)?),
            field: field.clone(),
            span: span.clone(),
        }),
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let scrutinee = Box::new(expand_inline_calls(scrutinee, inline_defs)?);
            let mut out_arms = Vec::new();
            for arm in arms {
                out_arms.push(MatchArm {
                    pat: arm.pat.clone(),
                    body: expand_inline_calls(&arm.body, inline_defs)?,
                    span: arm.span.clone(),
                });
            }
            Ok(Expr::Match {
                scrutinee,
                arms: out_arms,
                span: span.clone(),
            })
        }
        Expr::VecLit { elems, span, ann } => {
            let mut out = Vec::new();
            for el in elems {
                out.push(expand_inline_calls(el, inline_defs)?);
            }
            Ok(Expr::VecLit {
                elems: out,
                span: span.clone(),
                ann: ann.clone(),
            })
        }
        Expr::SetLit { elems, span, ann } => {
            let mut out = Vec::new();
            for el in elems {
                out.push(expand_inline_calls(el, inline_defs)?);
            }
            Ok(Expr::SetLit {
                elems: out,
                span: span.clone(),
                ann: ann.clone(),
            })
        }
        Expr::MapLit {
            kind,
            entries,
            span,
            ann,
        } => {
            let mut out_entries = Vec::new();
            for (k, v) in entries {
                out_entries.push((
                    expand_inline_calls(k, inline_defs)?,
                    expand_inline_calls(v, inline_defs)?,
                ));
            }
            Ok(Expr::MapLit {
                kind: kind.clone(),
                entries: out_entries,
                span: span.clone(),
                ann: ann.clone(),
            })
        }
        Expr::Ascribe { expr, ann, span } => Ok(Expr::Ascribe {
            expr: Box::new(expand_inline_calls(expr, inline_defs)?),
            ann: ann.clone(),
            span: span.clone(),
        }),
        Expr::Cast { expr, ann, span } => Ok(Expr::Cast {
            expr: Box::new(expand_inline_calls(expr, inline_defs)?),
            ann: ann.clone(),
            span: span.clone(),
        }),
        Expr::Int(..)
        | Expr::Float(..)
        | Expr::Str(..)
        | Expr::Bool(..)
        | Expr::Unit(..)
        | Expr::Keyword(..)
        | Expr::Var(..)
        | Expr::Continue { .. } => Ok(expr.clone()),
    }
}

fn inline_subst_local(expr: &Expr, map: &BTreeMap<String, Expr>) -> Expr {
    match expr {
        Expr::Var(name, _) => map.get(name).cloned().unwrap_or_else(|| expr.clone()),
        Expr::Ascribe { expr, ann, span } => Expr::Ascribe {
            expr: Box::new(inline_subst_local(expr, map)),
            ann: ann.clone(),
            span: span.clone(),
        },
        Expr::Cast { expr, ann, span } => Expr::Cast {
            expr: Box::new(inline_subst_local(expr, map)),
            ann: ann.clone(),
            span: span.clone(),
        },
        Expr::Int(..)
        | Expr::Float(..)
        | Expr::Str(..)
        | Expr::Bool(..)
        | Expr::Unit(..)
        | Expr::Keyword(..)
        | Expr::Continue { .. } => expr.clone(),
        Expr::Pair { key, val, span } => Expr::Pair {
            key: Box::new(inline_subst_local(key, map)),
            val: Box::new(inline_subst_local(val, map)),
            span: span.clone(),
        },
        Expr::Let {
            bindings,
            body,
            span,
        } => {
            let mut out = Vec::new();
            let mut shadowed = map.clone();
            for b in bindings {
                let expr = inline_subst_local(&b.expr, &shadowed);
                shadowed.remove(&b.rust_name);
                out.push(crate::ast::LetBinding {
                    name: b.name.clone(),
                    rust_name: b.rust_name.clone(),
                    ann: b.ann.clone(),
                    expr,
                    span: b.span.clone(),
                });
            }
            Expr::Let {
                bindings: out,
                body: Box::new(inline_subst_local(body, &shadowed)),
                span: span.clone(),
            }
        }
        Expr::Lambda { params, body, span } => {
            let mut shadowed = map.clone();
            for p in params {
                shadowed.remove(&p.rust_name);
            }
            Expr::Lambda {
                params: params.clone(),
                body: Box::new(inline_subst_local(body, &shadowed)),
                span: span.clone(),
            }
        }
        Expr::Do { exprs, span } => Expr::Do {
            exprs: exprs.iter().map(|e| inline_subst_local(e, map)).collect(),
            span: span.clone(),
        },
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => Expr::If {
            cond: Box::new(inline_subst_local(cond, map)),
            then_br: Box::new(inline_subst_local(then_br, map)),
            else_br: else_br
                .as_ref()
                .map(|b| Box::new(inline_subst_local(b, map))),
            span: span.clone(),
        },
        Expr::Loop { body, span } => Expr::Loop {
            body: Box::new(inline_subst_local(body, map)),
            span: span.clone(),
        },
        Expr::While { cond, body, span } => Expr::While {
            cond: Box::new(inline_subst_local(cond, map)),
            body: Box::new(inline_subst_local(body, map)),
            span: span.clone(),
        },
        Expr::For {
            var,
            iter,
            body,
            span,
        } => Expr::For {
            var: var.clone(),
            iter: inline_subst_iterable(iter, map),
            body: Box::new(inline_subst_local(body, map)),
            span: span.clone(),
        },
        Expr::Set { name, expr, span } => Expr::Set {
            name: name.clone(),
            expr: Box::new(inline_subst_local(expr, map)),
            span: span.clone(),
        },
        Expr::Break { value, span } => Expr::Break {
            value: value.as_ref().map(|v| Box::new(inline_subst_local(v, map))),
            span: span.clone(),
        },
        Expr::Field { base, field, span } => Expr::Field {
            base: Box::new(inline_subst_local(base, map)),
            field: field.clone(),
            span: span.clone(),
        },
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let scrutinee = Box::new(inline_subst_local(scrutinee, map));
            let mut out_arms = Vec::new();
            for arm in arms {
                out_arms.push(MatchArm {
                    pat: arm.pat.clone(),
                    body: inline_subst_local(&arm.body, map),
                    span: arm.span.clone(),
                });
            }
            Expr::Match {
                scrutinee,
                arms: out_arms,
                span: span.clone(),
            }
        }
        Expr::Call { op, args, span } => Expr::Call {
            op: op.clone(),
            args: args.iter().map(|a| inline_subst_local(a, map)).collect(),
            span: span.clone(),
        },
        Expr::CallDyn { func, args, span } => Expr::CallDyn {
            func: Box::new(inline_subst_local(func, map)),
            args: args.iter().map(|a| inline_subst_local(a, map)).collect(),
            span: span.clone(),
        },
        Expr::MethodCall {
            base,
            method,
            args,
            span,
        } => Expr::MethodCall {
            base: Box::new(inline_subst_local(base, map)),
            method: method.clone(),
            args: args.iter().map(|a| inline_subst_local(a, map)).collect(),
            span: span.clone(),
        },
        Expr::VecLit { elems, span, ann } => Expr::VecLit {
            elems: elems.iter().map(|e| inline_subst_local(e, map)).collect(),
            span: span.clone(),
            ann: ann.clone(),
        },
        Expr::MapLit {
            kind,
            entries,
            span,
            ann,
        } => {
            let entries = entries
                .iter()
                .map(|(k, v)| (inline_subst_local(k, map), inline_subst_local(v, map)))
                .collect();
            Expr::MapLit {
                kind: kind.clone(),
                entries,
                span: span.clone(),
                ann: ann.clone(),
            }
        }
        Expr::SetLit { elems, span, ann } => Expr::SetLit {
            elems: elems.iter().map(|e| inline_subst_local(e, map)).collect(),
            span: span.clone(),
            ann: ann.clone(),
        },
    }
}

fn inline_subst_range(
    map: &BTreeMap<String, Expr>,
    range: &crate::ast::RangeExpr,
) -> crate::ast::RangeExpr {
    crate::ast::RangeExpr {
        start: Box::new(inline_subst_local(&range.start, map)),
        end: Box::new(inline_subst_local(&range.end, map)),
        step: range
            .step
            .as_ref()
            .map(|s| Box::new(inline_subst_local(s, map))),
        inclusive: range.inclusive,
        span: range.span.clone(),
    }
}

fn inline_subst_range_local(
    range: &crate::ast::RangeExpr,
    inline_defs: &BTreeMap<String, crate::ast::InlineDef>,
) -> DslResult<crate::ast::RangeExpr> {
    let start = expand_inline_calls(&range.start, inline_defs)?;
    let end = expand_inline_calls(&range.end, inline_defs)?;
    let step = match &range.step {
        Some(s) => Some(Box::new(expand_inline_calls(s, inline_defs)?)),
        None => None,
    };
    Ok(crate::ast::RangeExpr {
        start: Box::new(start),
        end: Box::new(end),
        step,
        inclusive: range.inclusive,
        span: range.span.clone(),
    })
}

fn expr_ty<'a>(types: &'a BTreeMap<SpanKey, Ty>, expr: &Expr) -> Option<&'a Ty> {
    types.get(&SpanKey::new(&expr.span()))
}

fn add_bound(bounds: &mut BTreeMap<u32, Vec<GenericBound>>, id: u32, bound: GenericBound) {
    let entry = bounds.entry(id).or_insert_with(Vec::new);
    if !entry.iter().any(|b| *b == bound) {
        entry.push(bound);
    }
}

fn collect_generic_ids(ty: &Ty, out: &mut Vec<u32>) {
    match ty {
        Ty::Generic(id) => out.push(*id),
        Ty::Vec(inner) | Ty::Set(inner) | Ty::Option(inner) => collect_generic_ids(inner, out),
        Ty::Result(ok, err) => {
            collect_generic_ids(ok, out);
            collect_generic_ids(err, out);
        }
        Ty::Map(_, k, v) => {
            collect_generic_ids(k, out);
            collect_generic_ids(v, out);
        }
        Ty::Union(items) => {
            for item in items {
                collect_generic_ids(item, out);
            }
        }
        Ty::Named(_) | Ty::Unknown => {}
    }
}

fn collect_infer_var_ids(ty: &InferTy, out: &mut BTreeSet<u32>) {
    match ty {
        InferTy::Var(id) => {
            out.insert(*id);
        }
        InferTy::Vec(inner) | InferTy::Set(inner) | InferTy::Option(inner) => {
            collect_infer_var_ids(inner, out)
        }
        InferTy::Result(ok, err) => {
            collect_infer_var_ids(ok, out);
            collect_infer_var_ids(err, out);
        }
        InferTy::Map(_, k, v) => {
            collect_infer_var_ids(k, out);
            collect_infer_var_ids(v, out);
        }
        InferTy::Fn(params, ret) => {
            for p in params {
                collect_infer_var_ids(p, out);
            }
            collect_infer_var_ids(ret, out);
        }
        InferTy::Named(_) => {}
    }
}

fn add_bounds_for_ty(ty: &Ty, bounds: &mut BTreeMap<u32, Vec<GenericBound>>, bound: GenericBound) {
    let mut ids = Vec::new();
    collect_generic_ids(ty, &mut ids);
    for id in ids {
        add_bound(bounds, id, bound.clone());
    }
}

fn collect_bounds_expr(
    expr: &Expr,
    types: &BTreeMap<SpanKey, Ty>,
    auto_clones: &BTreeSet<SpanKey>,
    bounds: &mut BTreeMap<u32, Vec<GenericBound>>,
) {
    match expr {
        Expr::Var(_, sp) => {
            if auto_clones.contains(&SpanKey::new(sp)) {
                if let Some(ty) = expr_ty(types, expr) {
                    add_bounds_for_ty(ty, bounds, GenericBound::Clone);
                }
            }
        }
        Expr::Ascribe { expr, .. } => {
            collect_bounds_expr(expr, types, auto_clones, bounds);
        }
        Expr::Cast { expr, .. } => {
            collect_bounds_expr(expr, types, auto_clones, bounds);
        }
        Expr::Call { op, args, .. } => {
            match op.as_str() {
                "+" | "darcy.op/add" => {
                    for arg in args {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::Add);
                            add_bounds_for_ty(ty, bounds, GenericBound::Copy);
                        }
                    }
                }
                "-" | "darcy.op/sub" => {
                    for arg in args {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::Sub);
                            add_bounds_for_ty(ty, bounds, GenericBound::Copy);
                        }
                    }
                }
                "*" | "darcy.op/mul" => {
                    for arg in args {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::Mul);
                            add_bounds_for_ty(ty, bounds, GenericBound::Copy);
                        }
                    }
                }
                "/" | "darcy.op/div" => {
                    for arg in args {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::Div);
                            add_bounds_for_ty(ty, bounds, GenericBound::Copy);
                        }
                    }
                }
                "mod" | "darcy.op/mod" => {
                    for arg in args {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::Copy);
                        }
                    }
                }
                "darcy.op/eq" => {
                    for arg in args {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::PartialEq);
                        }
                    }
                }
                "darcy.op/lt"
                | "darcy.op/gt"
                | "darcy.op/lte"
                | "darcy.op/gte"
                | "<"
                | ">"
                | "<="
                | ">=" => {
                    for arg in args {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::PartialOrd);
                        }
                    }
                }
                "=" => {
                    for arg in args {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::PartialEq);
                        }
                    }
                }
                "darcy.core/clone" => {
                    if let Some(arg) = args.get(0) {
                        if let Some(ty) = expr_ty(types, arg) {
                            add_bounds_for_ty(ty, bounds, GenericBound::Clone);
                        }
                    }
                }
                _ => {}
            }
            for arg in args {
                collect_bounds_expr(arg, types, auto_clones, bounds);
            }
        }
        Expr::MethodCall {
            base, method, args, ..
        } => {
            if let Some(base_ty) = expr_ty(types, base) {
                match (base_ty, method.as_str()) {
                    (Ty::Generic(id), "len") => add_bound(bounds, *id, GenericBound::Len),
                    (Ty::Generic(id), "is_empty") => add_bound(bounds, *id, GenericBound::IsEmpty),
                    (Ty::Generic(id), "push") => {
                        if let Some(arg) = args.get(0) {
                            if let Some(arg_ty) = expr_ty(types, arg) {
                                add_bound(bounds, *id, GenericBound::Push(arg_ty.clone()));
                            }
                        }
                    }
                    _ => {}
                }
            }
            collect_bounds_expr(base, types, auto_clones, bounds);
            for arg in args {
                collect_bounds_expr(arg, types, auto_clones, bounds);
            }
        }
        Expr::Let { bindings, body, .. } => {
            for b in bindings {
                collect_bounds_expr(&b.expr, types, auto_clones, bounds);
            }
            collect_bounds_expr(body, types, auto_clones, bounds);
        }
        Expr::Lambda { body, .. } => {
            collect_bounds_expr(body, types, auto_clones, bounds);
        }
        Expr::CallDyn { func, args, .. } => {
            collect_bounds_expr(func, types, auto_clones, bounds);
            for arg in args {
                collect_bounds_expr(arg, types, auto_clones, bounds);
            }
        }
        Expr::Do { exprs, .. } => {
            for ex in exprs {
                collect_bounds_expr(ex, types, auto_clones, bounds);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_bounds_expr(cond, types, auto_clones, bounds);
            collect_bounds_expr(then_br, types, auto_clones, bounds);
            if let Some(else_br) = else_br {
                collect_bounds_expr(else_br, types, auto_clones, bounds);
            }
        }
        Expr::Loop { body, .. } | Expr::While { body, .. } => {
            collect_bounds_expr(body, types, auto_clones, bounds);
        }
        Expr::For { iter, body, .. } => {
            if let crate::ast::Iterable::Expr(ex) = iter {
                collect_bounds_expr(ex, types, auto_clones, bounds);
            }
            collect_bounds_expr(body, types, auto_clones, bounds);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_bounds_expr(scrutinee, types, auto_clones, bounds);
            for arm in arms {
                collect_bounds_expr(&arm.body, types, auto_clones, bounds);
            }
        }
        Expr::VecLit { elems, .. } | Expr::SetLit { elems, .. } => {
            for e in elems {
                collect_bounds_expr(e, types, auto_clones, bounds);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_bounds_expr(k, types, auto_clones, bounds);
                collect_bounds_expr(v, types, auto_clones, bounds);
            }
        }
        Expr::Pair { key, val, .. } => {
            collect_bounds_expr(key, types, auto_clones, bounds);
            collect_bounds_expr(val, types, auto_clones, bounds);
        }
        Expr::Break { value, .. } => {
            if let Some(v) = value {
                collect_bounds_expr(v, types, auto_clones, bounds);
            }
        }
        Expr::Set { expr, .. } => {
            collect_bounds_expr(expr, types, auto_clones, bounds);
        }
        Expr::Int(..) => {
            if let Some(Ty::Generic(id)) = expr_ty(types, expr) {
                add_bound(bounds, *id, GenericBound::FromInt);
            }
        }
        Expr::Float(..)
        | Expr::Str(..)
        | Expr::Bool(..)
        | Expr::Unit(..)
        | Expr::Keyword(..)
        | Expr::Continue { .. }
        | Expr::Field { .. } => {}
    }
}

fn collect_mutated_vars(expr: &Expr, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Set { name, expr, .. } => {
            out.insert(name.clone());
            collect_mutated_vars(expr, out);
        }
        Expr::Let { bindings, body, .. } => {
            for b in bindings {
                collect_mutated_vars(&b.expr, out);
            }
            collect_mutated_vars(body, out);
        }
        Expr::Lambda { body, .. } => {
            collect_mutated_vars(body, out);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_mutated_vars(arg, out);
            }
        }
        Expr::CallDyn { func, args, .. } => {
            collect_mutated_vars(func, out);
            for arg in args {
                collect_mutated_vars(arg, out);
            }
        }
        Expr::MethodCall { base, args, .. } => {
            collect_mutated_vars(base, out);
            for arg in args {
                collect_mutated_vars(arg, out);
            }
        }
        Expr::Do { exprs, .. } => {
            for ex in exprs {
                collect_mutated_vars(ex, out);
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            collect_mutated_vars(cond, out);
            collect_mutated_vars(then_br, out);
            if let Some(else_br) = else_br {
                collect_mutated_vars(else_br, out);
            }
        }
        Expr::Loop { body, .. } | Expr::While { body, .. } => {
            collect_mutated_vars(body, out);
        }
        Expr::For { iter, body, .. } => {
            if let crate::ast::Iterable::Expr(ex) = iter {
                collect_mutated_vars(ex, out);
            }
            collect_mutated_vars(body, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_mutated_vars(scrutinee, out);
            for arm in arms {
                collect_mutated_vars(&arm.body, out);
            }
        }
        Expr::VecLit { elems, .. } | Expr::SetLit { elems, .. } => {
            for e in elems {
                collect_mutated_vars(e, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_mutated_vars(k, out);
                collect_mutated_vars(v, out);
            }
        }
        Expr::Pair { key, val, .. } => {
            collect_mutated_vars(key, out);
            collect_mutated_vars(val, out);
        }
        Expr::Break { value, .. } => {
            if let Some(v) = value {
                collect_mutated_vars(v, out);
            }
        }
        Expr::Ascribe { expr, .. } => {
            collect_mutated_vars(expr, out);
        }
        Expr::Cast { expr, .. } => {
            collect_mutated_vars(expr, out);
        }
        Expr::Field { base, .. } => {
            collect_mutated_vars(base, out);
        }
        Expr::Var(..)
        | Expr::Int(..)
        | Expr::Float(..)
        | Expr::Str(..)
        | Expr::Bool(..)
        | Expr::Unit(..)
        | Expr::Keyword(..)
        | Expr::Continue { .. } => {}
    }
}

pub fn typecheck_fn(
    env: &TypeEnv,
    fns: &FnEnv,
    global_defs: &BTreeMap<String, Ty>,
    def_base_names: &BTreeSet<String>,
    f: &FnDef,
) -> DslResult<TypedFn> {
    let mut default_modes = BTreeMap::new();
    for p in &f.params {
        default_modes.insert(p.rust_name.clone(), ParamMode::ByVal);
    }
    if f.extern_ {
        let mut param_tys = BTreeMap::new();
        for p in &f.params {
            let ann = p.ann.clone().ok_or_else(|| {
                Diag::new(format!(
                    "extern function parameter '{}' must declare a type",
                    p.name
                ))
                .with_span(p.span.clone())
            })?;
            param_tys.insert(p.rust_name.clone(), ann);
        }
        let ret = f.extern_ret.clone().ok_or_else(|| {
            Diag::new("extern function must declare return type").with_span(f.span.clone())
        })?;
        let body = TypedExpr {
            expr: f.body.clone(),
            ty: ret,
            casts: vec![],
            types: BTreeMap::new(),
        };
        let mutated = BTreeSet::new();
        return Ok(TypedFn {
            def: f.clone(),
            param_tys,
            param_modes: default_modes,
            generic_bounds: BTreeMap::new(),
            body,
            mutated,
        });
    }

    let mut ctx = InferCtx::new();
    let field_env = build_field_infer_env(env, &mut ctx)?;
    let mut infer_fns = InferFnEnv::new();
    for (name, sigs) in &fns.fns {
        for sig in sigs {
            let params = sig
                .params
                .iter()
                .map(|t| infer_from_ty(&mut ctx, t))
                .collect();
            let ret = infer_from_ty(&mut ctx, &sig.ret);
            infer_fns.insert(name.clone(), InferFnSig { params, ret })?;
        }
    }
    let mut globals: BTreeMap<String, InferTy> = BTreeMap::new();
    for (name, ty) in global_defs {
        globals.insert(name.clone(), infer_from_ty(&mut ctx, ty));
    }

    check_param_bindings(f, def_base_names)?;
    let mut params = Vec::new();
    for p in &f.params {
        let ty = match &p.ann {
            Some(ann) => infer_from_ty(&mut ctx, ann),
            None => ctx.fresh_var(),
        };
        params.push(ty);
    }
    let sig = InferFnSig {
        params,
        ret: ctx.fresh_var(),
    };
    if infer_fns.get_arity(&f.name, f.params.len()).is_none() {
        infer_fns.insert(f.name.clone(), sig.clone())?;
    }
    let (param_tys, param_spans) = build_param_maps(f, &sig)?;
    apply_param_field_constraints(env, f, &param_tys, &param_spans, &mut ctx)?;

    let infer_body = infer_expr_type(
        env,
        &field_env,
        &infer_fns,
        &globals,
        def_base_names,
        &mut ctx,
        &param_tys,
        Some(&f.name),
        &f.body,
    )?;
    ctx.unify(&sig.ret, &infer_body.ty, &f.body.span())?;
    let mut keep_vars = BTreeSet::new();
    for ty in param_tys.values() {
        let resolved = ctx.resolve(ty);
        collect_infer_var_ids(&resolved, &mut keep_vars);
    }
    check_numeric_constraints(&mut ctx, &keep_vars)?;
    check_int_constraints(&mut ctx, &keep_vars)?;
    let mut keep_vars = BTreeSet::new();
    for ty in param_tys.values() {
        let resolved = ctx.resolve(ty);
        collect_infer_var_ids(&resolved, &mut keep_vars);
    }
    check_numeric_constraints(&mut ctx, &keep_vars)?;
    check_int_constraints(&mut ctx, &keep_vars)?;
    let mut final_param_tys = BTreeMap::new();
    for (name, ty) in &param_tys {
        let resolved = infer_to_ty_allow_generic(&ctx, ty);
        final_param_tys.insert(name.clone(), resolved);
    }

    let body = finalize_infer_expr_allow_generic(&ctx, infer_body).map_err(|mut d| {
        if d.span.is_none() {
            d = d.with_span(f.body.span());
        }
        d
    })?;

    let mut bounds = BTreeMap::new();
    collect_bounds_expr(&body.expr, &body.types, &BTreeSet::new(), &mut bounds);
    let mut mutated = BTreeSet::new();
    collect_mutated_vars(&body.expr, &mut mutated);
    let mut mutated = BTreeSet::new();
    collect_mutated_vars(&body.expr, &mut mutated);
    Ok(TypedFn {
        def: f.clone(),
        param_tys: final_param_tys,
        param_modes: default_modes,
        generic_bounds: bounds,
        body,
        mutated,
    })
}

#[allow(dead_code)]
fn typecheck_def(
    env: &TypeEnv,
    fns: &FnEnv,
    global_defs: &BTreeMap<String, Ty>,
    def_base_names: &BTreeSet<String>,
    d: &Def,
) -> DslResult<TypedDef> {
    let mut ctx = InferCtx::new();
    let field_env = build_field_infer_env(env, &mut ctx)?;
    let mut infer_fns = InferFnEnv::new();
    for (name, sigs) in &fns.fns {
        for sig in sigs {
            let params = sig
                .params
                .iter()
                .map(|t| infer_from_ty(&mut ctx, t))
                .collect();
            let ret = infer_from_ty(&mut ctx, &sig.ret);
            infer_fns.insert(name.clone(), InferFnSig { params, ret })?;
        }
    }
    let mut globals: BTreeMap<String, InferTy> = BTreeMap::new();
    for (name, ty) in global_defs {
        globals.insert(name.clone(), infer_from_ty(&mut ctx, ty));
    }
    let vars: BTreeMap<String, InferTy> = BTreeMap::new();
    let mut infer_body = infer_expr_type(
        env,
        &field_env,
        &infer_fns,
        &globals,
        def_base_names,
        &mut ctx,
        &vars,
        None,
        &d.expr,
    )?;
    if let Some(ann) = &d.ann {
        let ann_ty = infer_from_ty(&mut ctx, ann);
        ctx.unify(&ann_ty, &infer_body.ty, &d.span)?;
        infer_body.ty = ann_ty;
    }
    check_numeric_constraints(&mut ctx, &BTreeSet::new())?;
    check_int_constraints(&mut ctx, &BTreeSet::new())?;
    let body = finalize_infer_expr(&ctx, infer_body).map_err(|mut diag| {
        if diag.span.is_none() {
            diag = diag.with_span(d.expr.span());
        }
        diag
    })?;
    Ok(TypedDef {
        def: d.clone(),
        body,
    })
}

fn collect_field_constraints(expr: &Expr, out: &mut Vec<(String, String)>) {
    match expr {
        Expr::Field { base, field, .. } => {
            if let Expr::Var(v, _) = base.as_ref() {
                out.push((v.clone(), field.clone()));
            }
            collect_field_constraints(base, out);
        }
        Expr::Call { args, .. } => {
            for a in args {
                collect_field_constraints(a, out);
            }
        }
        _ => {}
    }
}

fn check_param_bindings(f: &FnDef, def_base_names: &BTreeSet<String>) -> DslResult<()> {
    let mut seen = BTreeSet::new();
    for p in &f.params {
        if !seen.insert(p.rust_name.clone()) {
            return Err(
                Diag::new(format!("duplicate parameter '{}'", p.name)).with_span(p.span.clone())
            );
        }
        if def_base_names.contains(&p.rust_name) {
            return Err(
                Diag::new(format!("parameter '{}' shadows a def name", p.name))
                    .with_span(p.span.clone()),
            );
        }
    }
    Ok(())
}

fn build_param_maps(
    f: &FnDef,
    sig: &InferFnSig,
) -> DslResult<(BTreeMap<String, InferTy>, BTreeMap<String, Span>)> {
    if sig.params.len() != f.params.len() {
        return Err(
            Diag::new("internal error: function parameter arity mismatch")
                .with_span(f.span.clone()),
        );
    }
    let mut param_tys = BTreeMap::new();
    let mut param_spans = BTreeMap::new();
    for (p, ty) in f.params.iter().zip(sig.params.iter()) {
        param_tys.insert(p.rust_name.clone(), ty.clone());
        param_spans.insert(p.rust_name.clone(), p.span.clone());
    }
    Ok((param_tys, param_spans))
}

fn apply_param_field_constraints(
    env: &TypeEnv,
    f: &FnDef,
    param_tys: &BTreeMap<String, InferTy>,
    param_spans: &BTreeMap<String, Span>,
    ctx: &mut InferCtx,
) -> DslResult<()> {
    let mut constraints: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut seen = Vec::new();
    collect_field_constraints(&f.body, &mut seen);
    for (p, field) in seen {
        constraints.entry(p).or_default().insert(field);
    }

    for (p, fields) in constraints {
        if !param_tys.contains_key(&p) {
            return Err(Diag::new(format!("unknown variable '{}'", p)).with_span(f.body.span()));
        }
        let cur = ctx.resolve(param_tys.get(&p).unwrap());
        match cur {
            InferTy::Named(ty_name) => {
                let sd = env.structs.get(&ty_name).ok_or_else(|| {
                    Diag::new(format!("unknown type '{}'", ty_name))
                        .with_span(param_spans[&p].clone())
                })?;
                for fld in &fields {
                    if !sd.fields.iter().any(|ff| ff.rust_name == *fld) {
                        return Err(Diag::new(format!(
                            "type '{}' has no field '{}'",
                            ty_name, fld
                        ))
                        .with_span(param_spans[&p].clone()));
                    }
                }
                continue;
            }
            InferTy::Vec(inner) => {
                let inner = ctx.resolve(&inner);
                let ty_name = match inner {
                    InferTy::Named(n) => n,
                    InferTy::Var(_) => {
                        return Err(Diag::new("cannot access field on unknown vector type")
                            .with_span(param_spans[&p].clone()))
                    }
                    InferTy::Vec(_) => {
                        return Err(Diag::new("cannot access field on nested vector type")
                            .with_span(param_spans[&p].clone()))
                    }
                    InferTy::Option(_)
                    | InferTy::Result(_, _)
                    | InferTy::Set(_)
                    | InferTy::Map(_, _, _)
                    | InferTy::Fn(_, _) => {
                        return Err(Diag::new("cannot access field on non-struct vector type")
                            .with_span(param_spans[&p].clone()))
                    }
                };
                let sd = env.structs.get(&ty_name).ok_or_else(|| {
                    Diag::new(format!("unknown type '{}'", ty_name))
                        .with_span(param_spans[&p].clone())
                })?;
                for fld in &fields {
                    if !sd.fields.iter().any(|ff| ff.rust_name == *fld) {
                        return Err(Diag::new(format!(
                            "type '{}' has no field '{}'",
                            ty_name, fld
                        ))
                        .with_span(param_spans[&p].clone()));
                    }
                }
                continue;
            }
            InferTy::Option(_)
            | InferTy::Result(_, _)
            | InferTy::Set(_)
            | InferTy::Map(_, _, _)
            | InferTy::Fn(_, _) => {
                return Err(Diag::new("cannot access field on non-struct type")
                    .with_span(param_spans[&p].clone()))
            }
            InferTy::Var(_) => {}
        }

        let mut candidates = Vec::new();
        for (name, sd) in &env.structs {
            let ok = fields
                .iter()
                .all(|fld| sd.fields.iter().any(|ff| &ff.rust_name == fld));
            if ok {
                candidates.push(name.clone());
            }
        }

        if candidates.is_empty() {
            return Err(Diag::new(format!(
                "cannot infer type for '{}': no known struct contains fields {}",
                p,
                fmt_set(&fields)
            ))
            .with_span(param_spans[&p].clone()));
        }
        if candidates.len() > 1 {
            return Err(Diag::new(format!(
                "ambiguous type for '{}': matches {}. Add annotation like {}:{}",
                p,
                candidates.join(", "),
                p,
                candidates[0],
            ))
            .with_span(param_spans[&p].clone()));
        }
        let target = InferTy::Named(candidates[0].clone());
        ctx.unify(
            param_tys.get(&p).unwrap_or(&target),
            &target,
            &param_spans[&p],
        )?;
    }
    Ok(())
}

fn fmt_set(s: &BTreeSet<String>) -> String {
    let mut v: Vec<_> = s.iter().cloned().collect();
    v.sort();
    format!("{{{}}}", v.join(", "))
}

fn infer_union_from_match_arms(
    env: &TypeEnv,
    arms: &[MatchArm],
    span: &Span,
) -> DslResult<Option<String>> {
    let mut union_name: Option<String> = None;
    for arm in arms {
        if let MatchPat::Variant {
            name, span: psp, ..
        } = &arm.pat
        {
            let (u, _) = env.variants.get(name).ok_or_else(|| {
                Diag::new(format!("unknown variant '{}'", name)).with_span(psp.clone())
            })?;
            match &union_name {
                Some(cur) if cur != u => {
                    return Err(Diag::new(format!(
                        "variant '{}' does not belong to '{}'",
                        name, cur
                    ))
                    .with_span(psp.clone()));
                }
                None => union_name = Some(u.clone()),
                _ => {}
            }
        }
    }
    if union_name.is_none() {
        return Err(Diag::new("cannot infer union type from case").with_span(span.clone()));
    }
    Ok(union_name)
}

#[derive(Debug, Clone)]
struct LoopFrame {
    result_ty: InferTy,
    saw_break: bool,
}

fn infer_expr_type(
    env: &TypeEnv,
    field_env: &FieldInferEnv,
    fns: &InferFnEnv,
    globals: &BTreeMap<String, InferTy>,
    def_base_names: &BTreeSet<String>,
    ctx: &mut InferCtx,
    vars: &BTreeMap<String, InferTy>,
    current_fn: Option<&str>,
    e: &Expr,
) -> DslResult<InferExpr> {
    let mut loop_stack = Vec::new();
    infer_expr_type_internal(
        env,
        field_env,
        fns,
        globals,
        def_base_names,
        ctx,
        vars,
        &mut loop_stack,
        current_fn,
        e,
    )
}

fn infer_expr_type_internal(
    env: &TypeEnv,
    field_env: &FieldInferEnv,
    fns: &InferFnEnv,
    globals: &BTreeMap<String, InferTy>,
    def_base_names: &BTreeSet<String>,
    ctx: &mut InferCtx,
    vars: &BTreeMap<String, InferTy>,
    loop_stack: &mut Vec<LoopFrame>,
    current_fn: Option<&str>,
    e: &Expr,
) -> DslResult<InferExpr> {
    match e {
        Expr::Int(_, sp) => {
            let ty = ctx.fresh_var();
            ctx.numeric_constraints.push((ty.clone(), sp.clone()));
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Float(_, sp) => {
            let ty = InferTy::Named("f64".to_string());
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Str(_, sp) => {
            let ty = InferTy::Named("string".to_string());
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Bool(_, sp) => {
            let ty = InferTy::Named("bool".to_string());
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Unit(sp) => {
            let ty = InferTy::Named("()".to_string());
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Keyword(_, sp) => {
            let ty = InferTy::Named("string".to_string());
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Var(v, sp) => {
            let ty = if let Some(local) = vars.get(v) {
                local.clone()
            } else if let Some(global) = globals.get(v) {
                global.clone()
            } else {
                return Err(Diag::new(format!("unknown variable '{}'", v)).with_span(sp.clone()));
            };
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Ascribe { expr, ann, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let te = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                expr,
            )?;
            casts.extend(te.casts.clone());
            types.extend(te.types);
            let ann_ty = infer_from_ty(ctx, ann);
            ctx.unify(&ann_ty, &te.ty, span)?;
            types.insert(SpanKey::new(span), ann_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: ann_ty,
                casts,
                types,
            })
        }
        Expr::Cast { expr, ann, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let te = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                expr,
            )?;
            casts.extend(te.casts.clone());
            types.extend(te.types);
            if !is_cast_target(ann) {
                return Err(Diag::new("cast target must be a numeric type").with_span(span.clone()));
            }
            let target_ty = infer_from_ty(ctx, ann);
            let resolved = ctx.resolve(&te.ty);
            match &resolved {
                InferTy::Named(name) => {
                    if !is_cast_source_name(name) {
                        return Err(Diag::new("cast source must be numeric or bool")
                            .with_span(span.clone()));
                    }
                }
                InferTy::Var(_) => {
                    ctx.numeric_constraints
                        .push((resolved.clone(), span.clone()));
                }
                _ => {
                    return Err(
                        Diag::new("cast source must be numeric or bool").with_span(span.clone())
                    );
                }
            }
            types.insert(SpanKey::new(span), target_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: target_ty,
                casts,
                types,
            })
        }
        Expr::VecLit { elems, span, ann } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut elem_tys = Vec::new();
            for el in elems {
                let te = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    el,
                )?;
                casts.extend(te.casts.clone());
                types.extend(te.types);
                elem_tys.push((te.ty, el.span()));
            }

            let elem_ty = match ann {
                Some(Ty::Vec(inner)) => {
                    let ann_ty = infer_from_ty(ctx, inner);
                    for (ty, el_sp) in &elem_tys {
                        ctx.unify(&ann_ty, ty, el_sp)?;
                    }
                    ann_ty
                }
                Some(_) => {
                    return Err(Diag::new("vector literal must use Vec<T> annotation")
                        .with_span(span.clone()));
                }
                None => {
                    if elem_tys.is_empty() {
                        return Err(Diag::new(
                            "cannot infer vector element type from empty literal; add Vec<T>",
                        )
                        .with_span(span.clone()));
                    }
                    let mut cur = elem_tys[0].0.clone();
                    for (ty, el_sp) in &elem_tys[1..] {
                        ctx.unify(&cur, ty, el_sp)?;
                        cur = ctx.resolve(&cur);
                    }
                    cur
                }
            };

            let ty = InferTy::Vec(Box::new(ctx.resolve(&elem_ty)));
            types.insert(SpanKey::new(span), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts,
                types,
            })
        }
        Expr::SetLit { elems, span, ann } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut elem_tys = Vec::new();
            for el in elems {
                let te = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    el,
                )?;
                casts.extend(te.casts.clone());
                types.extend(te.types);
                elem_tys.push((te.ty, el.span()));
            }

            let elem_ty = match ann {
                Some(Ty::Set(inner)) => {
                    let ann_ty = infer_from_ty(ctx, inner);
                    for (ty, el_sp) in &elem_tys {
                        ctx.unify(&ann_ty, ty, el_sp)?;
                    }
                    ann_ty
                }
                Some(_) => {
                    return Err(
                        Diag::new("set literal must use set<T> annotation").with_span(span.clone())
                    );
                }
                None => {
                    if elem_tys.is_empty() {
                        return Err(Diag::new(
                            "cannot infer set element type from empty literal; add set<T>",
                        )
                        .with_span(span.clone()));
                    }
                    let mut cur = elem_tys[0].0.clone();
                    for (ty, el_sp) in &elem_tys[1..] {
                        ctx.unify(&cur, ty, el_sp)?;
                        cur = ctx.resolve(&cur);
                    }
                    cur
                }
            };

            let ty = InferTy::Set(Box::new(ctx.resolve(&elem_ty)));
            types.insert(SpanKey::new(span), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts,
                types,
            })
        }

        Expr::Let {
            bindings,
            body,
            span,
        } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut local_vars = vars.clone();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for b in bindings {
                if !seen.insert(b.rust_name.clone()) {
                    return Err(Diag::new(format!("duplicate let binding '{}'", b.name))
                        .with_span(b.span.clone()));
                }
                if def_base_names.contains(&b.rust_name) {
                    return Err(
                        Diag::new(format!("let binding '{}' shadows a def name", b.name))
                            .with_span(b.span.clone()),
                    );
                }
                let te = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    &local_vars,
                    loop_stack,
                    current_fn,
                    &b.expr,
                )?;
                casts.extend(te.casts.clone());
                types.extend(te.types);
                let mut ty = te.ty;
                if let Some(ann) = &b.ann {
                    let ann_ty = infer_from_ty(ctx, ann);
                    ctx.unify(&ann_ty, &ty, &b.span)?;
                    ty = ann_ty;
                }
                local_vars.insert(b.rust_name.clone(), ty);
            }
            let tbody = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                &local_vars,
                loop_stack,
                current_fn,
                body,
            )?;
            casts.extend(tbody.casts.clone());
            types.extend(tbody.types.clone());
            let out_ty = tbody.ty.clone();
            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }
        Expr::Lambda {
            params,
            body,
            span: _,
        } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut param_tys = Vec::new();
            let mut local_vars = vars.clone();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for p in params {
                if !seen.insert(p.rust_name.clone()) {
                    return Err(Diag::new(format!("duplicate parameter '{}'", p.name))
                        .with_span(p.span.clone()));
                }
                if def_base_names.contains(&p.rust_name) {
                    return Err(
                        Diag::new(format!("parameter '{}' shadows a def name", p.name))
                            .with_span(p.span.clone()),
                    );
                }
                let ty = match &p.ann {
                    Some(ann) => infer_from_ty(ctx, ann),
                    None => ctx.fresh_var(),
                };
                param_tys.push(ty.clone());
                local_vars.insert(p.rust_name.clone(), ty);
            }
            let tbody = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                &local_vars,
                loop_stack,
                current_fn,
                body,
            )?;
            casts.extend(tbody.casts.clone());
            types.extend(tbody.types.clone());
            let out_ty = InferTy::Fn(param_tys, Box::new(tbody.ty.clone()));
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }
        Expr::CallDyn { func, args, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let tfunc = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                func,
            )?;
            casts.extend(tfunc.casts.clone());
            types.extend(tfunc.types);
            let mut arg_tys = Vec::new();
            for a in args {
                let ta = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    a,
                )?;
                casts.extend(ta.casts.clone());
                types.extend(ta.types.clone());
                arg_tys.push(ta.ty.clone());
            }
            let ret = ctx.fresh_var();
            let fn_ty = InferTy::Fn(arg_tys.clone(), Box::new(ret.clone()));
            ctx.unify(&tfunc.ty, &fn_ty, span)?;
            types.insert(SpanKey::new(span), ret.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: ret,
                casts,
                types,
            })
        }
        Expr::MethodCall {
            base,
            args,
            span,
            method,
        } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let tbase = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                base,
            )?;
            casts.extend(tbase.casts.clone());
            types.extend(tbase.types);
            let mut targs = Vec::new();
            for a in args {
                let ta = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    a,
                )?;
                casts.extend(ta.casts.clone());
                types.extend(ta.types.clone());
                targs.push(ta);
            }
            let mut ret = ctx.fresh_var();
            let base_ty = ctx.resolve(&tbase.ty);
            match (base_ty, method.as_str()) {
                (InferTy::Vec(_), "len") => {
                    if !targs.is_empty() {
                        return Err(Diag::new("len expects 0 arguments").with_span(span.clone()));
                    }
                    ret = InferTy::Named("usize".to_string());
                }
                (InferTy::Vec(_), "is_empty") => {
                    if !targs.is_empty() {
                        return Err(
                            Diag::new("is_empty expects 0 arguments").with_span(span.clone())
                        );
                    }
                    ret = InferTy::Named("bool".to_string());
                }
                (InferTy::Vec(inner), "push") => {
                    if targs.len() != 1 {
                        return Err(Diag::new("push expects 1 argument").with_span(span.clone()));
                    }
                    ctx.unify(&targs[0].ty, &inner, &args[0].span())?;
                    ret = InferTy::Named("()".to_string());
                }
                (InferTy::Named(n), "len") if n == "string" => {
                    if !targs.is_empty() {
                        return Err(Diag::new("len expects 0 arguments").with_span(span.clone()));
                    }
                    ret = InferTy::Named("usize".to_string());
                }
                (InferTy::Named(n), "is_empty") if n == "string" => {
                    if !targs.is_empty() {
                        return Err(
                            Diag::new("is_empty expects 0 arguments").with_span(span.clone())
                        );
                    }
                    ret = InferTy::Named("bool".to_string());
                }
                (InferTy::Var(_), "len") => {
                    if !targs.is_empty() {
                        return Err(Diag::new("len expects 0 arguments").with_span(span.clone()));
                    }
                    ret = InferTy::Named("usize".to_string());
                }
                (InferTy::Var(_), "is_empty") => {
                    if !targs.is_empty() {
                        return Err(
                            Diag::new("is_empty expects 0 arguments").with_span(span.clone())
                        );
                    }
                    ret = InferTy::Named("bool".to_string());
                }
                _ => {}
            }
            types.insert(SpanKey::new(span), ret.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: ret,
                casts,
                types,
            })
        }

        Expr::Do { exprs, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut last_ty = InferTy::Named("()".to_string());
            for ex in exprs {
                let te = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    ex,
                )?;
                casts.extend(te.casts.clone());
                types.extend(te.types.clone());
                last_ty = te.ty;
            }
            types.insert(SpanKey::new(span), last_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: last_ty,
                casts,
                types,
            })
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let tcond = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                cond,
            )?;
            casts.extend(tcond.casts.clone());
            types.extend(tcond.types.clone());
            let bool_ty = InferTy::Named("bool".to_string());
            ctx.unify(&tcond.ty, &bool_ty, &cond.span())?;

            let tthen = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                then_br,
            )?;
            casts.extend(tthen.casts.clone());
            types.extend(tthen.types.clone());

            let out_ty = if let Some(else_br) = else_br {
                let telse = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    else_br,
                )?;
                casts.extend(telse.casts.clone());
                types.extend(telse.types.clone());
                ctx.unify(&tthen.ty, &telse.ty, &else_br.span())?;
                ctx.resolve(&tthen.ty)
            } else {
                InferTy::Named("()".to_string())
            };

            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }
        Expr::Loop { body, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let result_ty = ctx.fresh_var();
            loop_stack.push(LoopFrame {
                result_ty: result_ty.clone(),
                saw_break: false,
            });
            let tbody = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                body,
            )?;
            casts.extend(tbody.casts.clone());
            types.extend(tbody.types.clone());
            let frame = loop_stack.pop().expect("loop frame");
            let out_ty = if frame.saw_break {
                ctx.resolve(&frame.result_ty)
            } else {
                InferTy::Named("()".to_string())
            };
            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }
        Expr::While { cond, body, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let tcond = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                cond,
            )?;
            casts.extend(tcond.casts.clone());
            types.extend(tcond.types.clone());
            let bool_ty = InferTy::Named("bool".to_string());
            ctx.unify(&tcond.ty, &bool_ty, &cond.span())?;

            let result_ty = ctx.fresh_var();
            loop_stack.push(LoopFrame {
                result_ty: result_ty.clone(),
                saw_break: false,
            });
            let tbody = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                body,
            )?;
            casts.extend(tbody.casts.clone());
            types.extend(tbody.types.clone());
            let frame = loop_stack.pop().expect("loop frame");
            let out_ty = if frame.saw_break {
                ctx.resolve(&frame.result_ty)
            } else {
                InferTy::Named("()".to_string())
            };
            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }
        Expr::For {
            var,
            iter,
            body,
            span,
        } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();

            let elem_ty = match iter {
                crate::ast::Iterable::Range(range) => {
                    let tstart = infer_expr_type_internal(
                        env,
                        field_env,
                        fns,
                        globals,
                        def_base_names,
                        ctx,
                        vars,
                        loop_stack,
                        current_fn,
                        &range.start,
                    )?;
                    let tend = infer_expr_type_internal(
                        env,
                        field_env,
                        fns,
                        globals,
                        def_base_names,
                        ctx,
                        vars,
                        loop_stack,
                        current_fn,
                        &range.end,
                    )?;
                    casts.extend(tstart.casts.clone());
                    casts.extend(tend.casts.clone());
                    types.extend(tstart.types.clone());
                    types.extend(tend.types.clone());
                    ctx.unify(&tstart.ty, &tend.ty, &range.end.span())?;
                    let mut elem_ty = ctx.resolve(&tstart.ty);
                    if let Some(step) = &range.step {
                        let tstep = infer_expr_type_internal(
                            env,
                            field_env,
                            fns,
                            globals,
                            def_base_names,
                            ctx,
                            vars,
                            loop_stack,
                            current_fn,
                            step,
                        )?;
                        casts.extend(tstep.casts.clone());
                        types.extend(tstep.types.clone());
                        ctx.unify(&elem_ty, &tstep.ty, &step.span())?;
                        elem_ty = ctx.resolve(&elem_ty);
                    }
                    let elem_resolved = ctx.resolve(&elem_ty);
                    match &elem_resolved {
                        InferTy::Var(_) => {
                            ctx.numeric_constraints
                                .push((elem_resolved.clone(), range.span.clone()));
                        }
                        _ => {
                            let elem_concrete =
                                infer_to_ty(ctx, &elem_resolved).ok_or_else(|| {
                                    Diag::new("cannot infer range element type")
                                        .with_span(range.span.clone())
                                })?;
                            if !is_numeric(&elem_concrete) {
                                return Err(Diag::new("range bounds must be numeric")
                                    .with_span(range.span.clone()));
                            }
                        }
                    }
                    elem_ty
                }
                crate::ast::Iterable::Expr(ex) => {
                    let titer = infer_expr_type_internal(
                        env,
                        field_env,
                        fns,
                        globals,
                        def_base_names,
                        ctx,
                        vars,
                        loop_stack,
                        current_fn,
                        ex,
                    )?;
                    casts.extend(titer.casts.clone());
                    types.extend(titer.types.clone());
                    let iter_ty = ctx.resolve(&titer.ty);
                    match iter_ty {
                        InferTy::Vec(inner) => *inner,
                        InferTy::Var(_) => {
                            let inner = ctx.fresh_var();
                            ctx.unify(
                                &titer.ty,
                                &InferTy::Vec(Box::new(inner.clone())),
                                &ex.span(),
                            )?;
                            inner
                        }
                        _ => {
                            return Err(Diag::new("for loop iterable must be a vector")
                                .with_span(ex.span()));
                        }
                    }
                }
            };

            let mut body_vars = vars.clone();
            body_vars.insert(var.clone(), elem_ty.clone());
            let result_ty = ctx.fresh_var();
            loop_stack.push(LoopFrame {
                result_ty: result_ty.clone(),
                saw_break: false,
            });
            let tbody = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                &body_vars,
                loop_stack,
                current_fn,
                body,
            )?;
            casts.extend(tbody.casts.clone());
            types.extend(tbody.types.clone());
            let frame = loop_stack.pop().expect("loop frame");
            let out_ty = if frame.saw_break {
                ctx.resolve(&frame.result_ty)
            } else {
                InferTy::Named("()".to_string())
            };
            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }
        Expr::Set { name, expr, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let texpr = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                expr,
            )?;
            casts.extend(texpr.casts.clone());
            types.extend(texpr.types.clone());
            if let Some(existing) = vars.get(name) {
                ctx.unify(existing, &texpr.ty, span)?;
            } else {
                return Err(
                    Diag::new(format!("unknown variable '{}'", name)).with_span(span.clone())
                );
            }
            let out_ty = InferTy::Named("()".to_string());
            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }

        Expr::Break { value, span } => {
            if loop_stack.is_empty() {
                return Err(Diag::new("break is only allowed inside loops").with_span(span.clone()));
            }
            let frame_idx = loop_stack.len() - 1;
            let result_ty = loop_stack[frame_idx].result_ty.clone();
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let value_ty = if let Some(v) = value {
                let tv = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    v,
                )?;
                casts.extend(tv.casts.clone());
                types.extend(tv.types.clone());
                tv.ty
            } else {
                InferTy::Named("()".to_string())
            };
            ctx.unify(&result_ty, &value_ty, span)?;
            loop_stack[frame_idx].saw_break = true;
            types.insert(SpanKey::new(span), result_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: result_ty.clone(),
                casts,
                types,
            })
        }
        Expr::Continue { span } => {
            if loop_stack.is_empty() {
                return Err(
                    Diag::new("continue is only allowed inside loops").with_span(span.clone())
                );
            }
            let out_ty = InferTy::Named("()".to_string());
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts: vec![],
                types,
            })
        }
        Expr::Pair { span, .. } => Err(Diag::new(
            "pair literal is only allowed inside hash-map/new or btree-map/new",
        )
        .with_span(span.clone())),
        Expr::MapLit {
            kind,
            entries,
            span,
            ann,
        } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut key_ty = None::<InferTy>;
            let mut val_ty = None::<InferTy>;

            if entries.is_empty() && ann.is_none() {
                return Err(Diag::new(
                    "cannot infer map type from empty literal; add hash-map<K,V> annotation",
                )
                .with_span(span.clone()));
            }

            if let Some((k, v)) = ann {
                key_ty = Some(infer_from_ty(ctx, k));
                val_ty = Some(infer_from_ty(ctx, v));
            }

            for (k, v) in entries {
                let tk = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    k,
                )?;
                let tv = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    v,
                )?;
                casts.extend(tk.casts.clone());
                casts.extend(tv.casts.clone());
                types.extend(tk.types);
                types.extend(tv.types);

                if let Some(ref cur_k) = key_ty {
                    ctx.unify(cur_k, &tk.ty, &k.span())?;
                } else {
                    key_ty = Some(tk.ty.clone());
                }
                if let Some(ref cur_v) = val_ty {
                    ctx.unify(cur_v, &tv.ty, &v.span())?;
                } else {
                    val_ty = Some(tv.ty.clone());
                }
            }

            let key_ty = key_ty.unwrap_or_else(|| ctx.fresh_var());
            let val_ty = val_ty.unwrap_or_else(|| ctx.fresh_var());
            let out_ty = InferTy::Map(kind.clone(), Box::new(key_ty), Box::new(val_ty));
            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }
        Expr::Field { base, field, span } => {
            let tb = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                base,
            )?;
            let base_ty = ctx.resolve(&tb.ty);
            let (struct_name, is_vec) = match base_ty {
                InferTy::Named(n) => (n, false),
                InferTy::Vec(inner) => match ctx.resolve(&inner) {
                    InferTy::Named(n) => (n, true),
                    InferTy::Var(_) => {
                        return Err(Diag::new("cannot access field on unknown vector type")
                            .with_span(span.clone()))
                    }
                    InferTy::Vec(_) => {
                        return Err(Diag::new("cannot access field on nested vector type")
                            .with_span(span.clone()))
                    }
                    InferTy::Option(_)
                    | InferTy::Result(_, _)
                    | InferTy::Map(_, _, _)
                    | InferTy::Fn(_, _) => {
                        return Err(Diag::new("cannot access field on non-struct vector type")
                            .with_span(span.clone()))
                    }
                    InferTy::Set(_) => {
                        return Err(
                            Diag::new("cannot access field on set type").with_span(span.clone())
                        )
                    }
                },
                InferTy::Var(_) => {
                    return Err(
                        Diag::new("cannot access field on unknown type").with_span(span.clone())
                    )
                }
                InferTy::Option(_)
                | InferTy::Result(_, _)
                | InferTy::Map(_, _, _)
                | InferTy::Fn(_, _) => {
                    return Err(
                        Diag::new("cannot access field on non-struct type").with_span(span.clone())
                    )
                }
                InferTy::Set(_) => {
                    return Err(Diag::new("cannot access field on set type").with_span(span.clone()))
                }
            };
            let sinfo = field_env.structs.get(&struct_name).ok_or_else(|| {
                Diag::new(format!("unknown type '{}'", struct_name)).with_span(span.clone())
            })?;
            let f = sinfo
                .fields
                .iter()
                .find(|ff| ff.rust_name == *field)
                .ok_or_else(|| {
                    Diag::new(format!("type '{}' has no field '{}'", struct_name, field))
                        .with_span(span.clone())
                })?;

            let field_ty = f.ty.clone();
            let ty = if is_vec {
                InferTy::Vec(Box::new(field_ty))
            } else {
                field_ty
            };
            let mut types = tb.types;
            types.insert(SpanKey::new(span), ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty,
                casts: tb.casts,
                types,
            })
        }
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let scrut = infer_expr_type_internal(
                env,
                field_env,
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                current_fn,
                scrutinee,
            )?;
            let union_name = match ctx.resolve(&scrut.ty) {
                InferTy::Named(n) => n,
                InferTy::Var(_) => {
                    let inferred = infer_union_from_match_arms(env, arms, span)?;
                    let inferred = inferred.ok_or_else(|| {
                        Diag::new("cannot infer union type from case").with_span(span.clone())
                    })?;
                    let target = InferTy::Named(inferred.clone());
                    ctx.unify(&scrut.ty, &target, span)?;
                    inferred
                }
                InferTy::Vec(_) => {
                    return Err(Diag::new("cannot case on vector type").with_span(span.clone()))
                }
                InferTy::Option(_)
                | InferTy::Result(_, _)
                | InferTy::Map(_, _, _)
                | InferTy::Fn(_, _) => {
                    return Err(Diag::new("cannot case on non-union type").with_span(span.clone()))
                }
                InferTy::Set(_) => todo!(),
            };
            if !env.unions.contains_key(&union_name) {
                return Err(Diag::new(format!("type '{}' is not a union", union_name))
                    .with_span(span.clone()));
            }

            let mut seen = BTreeSet::new();
            let mut has_wildcard = false;
            let mut casts = scrut.casts.clone();
            let mut types = scrut.types.clone();
            let mut out_ty: Option<InferTy> = None;

            for arm in arms {
                let mut arm_vars = vars.clone();
                match &arm.pat {
                    MatchPat::Variant {
                        name,
                        bindings,
                        span: psp,
                    } => {
                        let (u, _vdef) = env.variants.get(name).ok_or_else(|| {
                            Diag::new(format!("unknown variant '{}'", name)).with_span(psp.clone())
                        })?;
                        let vinf = field_env.variants.get(name).ok_or_else(|| {
                            Diag::new("internal error: missing variant inference")
                                .with_span(psp.clone())
                        })?;
                        if u != &union_name {
                            return Err(Diag::new(format!(
                                "variant '{}' does not belong to '{}'",
                                name, union_name
                            ))
                            .with_span(psp.clone()));
                        }
                        seen.insert(name.clone());
                        for (field, binding, bspan) in bindings {
                            let f = vinf
                                .fields
                                .iter()
                                .find(|ff| ff.rust_name == *field)
                                .ok_or_else(|| {
                                    Diag::new(format!(
                                        "variant '{}' has no field '{}'",
                                        name, field
                                    ))
                                    .with_span(bspan.clone())
                                })?;
                            if binding != "_" {
                                arm_vars.insert(binding.clone(), f.ty.clone());
                            }
                        }
                    }
                    MatchPat::Wildcard(_) => {
                        has_wildcard = true;
                    }
                }

                let tarm = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    &arm_vars,
                    loop_stack,
                    current_fn,
                    &arm.body,
                )?;
                casts.extend(tarm.casts.clone());
                types.extend(tarm.types.clone());
                if let Some(current) = &out_ty {
                    ctx.unify(current, &tarm.ty, &arm.body.span())?;
                } else {
                    out_ty = Some(tarm.ty.clone());
                }
            }

            let mut missing = Vec::new();
            let all_variants = &env.unions.get(&union_name).unwrap().variants;
            for v in all_variants {
                if !seen.contains(&v.name) {
                    missing.push(v.name.clone());
                }
            }

            let mut final_arms = arms.clone();
            if missing.is_empty() {
                if has_wildcard {
                    eprintln!("warning: redundant wildcard arm; all variants are already covered");
                    final_arms.retain(|arm| !matches!(arm.pat, MatchPat::Wildcard(_)));
                }
            } else {
                if !has_wildcard {
                    return Err(Diag::new(format!(
                        "non-exhaustive case, missing: {}",
                        missing.join(", ")
                    ))
                    .with_span(span.clone()));
                }
            }

            let e = if final_arms.len() != arms.len() {
                Expr::Match {
                    scrutinee: scrutinee.clone(),
                    arms: final_arms,
                    span: span.clone(),
                }
            } else {
                e.clone()
            };

            let out_ty = out_ty.unwrap_or_else(|| ctx.fresh_var());
            types.insert(SpanKey::new(span), out_ty.clone());
            Ok(InferExpr {
                expr: e.clone(),
                ty: out_ty,
                casts,
                types,
            })
        }
        Expr::Call { op, args, span } => {
            if op == "darcy.hash-map/new" || op == "darcy.btree-map/new" {
                let kind = if op == "darcy.hash-map/new" {
                    MapKind::Hash
                } else {
                    MapKind::BTree
                };
                if args.is_empty() {
                    return Err(Diag::new(
                        "cannot infer map type from empty literal; add hash-map<K,V> annotation",
                    )
                    .with_span(span.clone()));
                }
                let mut casts = Vec::new();
                let mut types = BTreeMap::new();
                let mut key_ty = None::<InferTy>;
                let mut val_ty = None::<InferTy>;
                for entry in args {
                    let (key, val) = match entry {
                        Expr::Pair { key, val, .. } => (key.as_ref(), val.as_ref()),
                        Expr::VecLit { elems, .. } if elems.len() == 2 => (&elems[0], &elems[1]),
                        _ => {
                            return Err(
                                Diag::new("map entry must be [key value]").with_span(entry.span())
                            )
                        }
                    };
                    let tk = infer_expr_type_internal(
                        env,
                        field_env,
                        fns,
                        globals,
                        def_base_names,
                        ctx,
                        vars,
                        loop_stack,
                        current_fn,
                        key,
                    )?;
                    let tv = infer_expr_type_internal(
                        env,
                        field_env,
                        fns,
                        globals,
                        def_base_names,
                        ctx,
                        vars,
                        loop_stack,
                        current_fn,
                        val,
                    )?;
                    casts.extend(tk.casts.clone());
                    casts.extend(tv.casts.clone());
                    types.extend(tk.types);
                    types.extend(tv.types);

                    if let Some(ref cur_k) = key_ty {
                        ctx.unify(cur_k, &tk.ty, &key.span())?;
                    } else {
                        key_ty = Some(tk.ty.clone());
                    }
                    if let Some(ref cur_v) = val_ty {
                        ctx.unify(cur_v, &tv.ty, &val.span())?;
                    } else {
                        val_ty = Some(tv.ty.clone());
                    }
                }

                let key_ty = key_ty.unwrap_or_else(|| ctx.fresh_var());
                let val_ty = val_ty.unwrap_or_else(|| ctx.fresh_var());
                let out_ty = InferTy::Map(kind, Box::new(key_ty), Box::new(val_ty));
                types.insert(SpanKey::new(span), out_ty.clone());
                return Ok(InferExpr {
                    expr: e.clone(),
                    ty: out_ty,
                    casts,
                    types,
                });
            }

            let mut targs = Vec::new();
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            for a in args {
                let ta = infer_expr_type_internal(
                    env,
                    field_env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
                    current_fn,
                    a,
                )?;
                casts.extend(ta.casts.clone());
                types.extend(ta.types.clone());
                targs.push(ta);
            }

            match op.as_str() {
                "darcy.io/dbg" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'dbg' expects 1 argument").with_span(span.clone()));
                    }
                    let out_ty = InferTy::Named("()".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.fmt/print" | "darcy.fmt/println" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'print' expects 1 argument").with_span(span.clone()));
                    }
                    let out_ty = InferTy::Named("()".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.fmt/format" | "darcy.fmt/pretty" => {
                    if targs.len() != 1 {
                        return Err(
                            Diag::new("'format' expects 1 argument").with_span(span.clone())
                        );
                    }
                    let out_ty = InferTy::Named("string".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.option/some" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'some' expects 1 argument").with_span(span.clone()));
                    }
                    let out_ty = InferTy::Option(Box::new(targs[0].ty.clone()));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.option/none" => {
                    if !targs.is_empty() {
                        return Err(Diag::new("'none' expects 0 arguments").with_span(span.clone()));
                    }
                    // Option<T> None creates an Option where T is unknown initially
                    let out_ty = InferTy::Option(Box::new(ctx.fresh_var()));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.option/is-some" | "darcy.option/is-none" => {
                    if targs.len() != 1 {
                        return Err(
                            Diag::new("'is-some' expects 1 argument").with_span(span.clone())
                        );
                    }
                    let _ = ensure_option_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.result/ok" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'ok' expects 1 argument").with_span(span.clone()));
                    }
                    // For Ok(T), the Err type is unknown, so use a fresh variable
                    let out_ty =
                        InferTy::Result(Box::new(targs[0].ty.clone()), Box::new(ctx.fresh_var()));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.result/err" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'err' expects 1 argument").with_span(span.clone()));
                    }
                    // For Err(E), the Ok type is unknown, so use a fresh variable
                    let out_ty =
                        InferTy::Result(Box::new(ctx.fresh_var()), Box::new(targs[0].ty.clone()));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }

                "darcy.result/is-ok" | "darcy.result/is-err" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'is-ok' expects 1 argument").with_span(span.clone()));
                    }
                    let _ = ensure_result_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.math/exp" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'exp' expects 1 argument").with_span(span.clone()));
                    }
                    let target = InferTy::Named("f64".to_string());
                    ctx.unify(&targs[0].ty, &target, &args[0].span())?;
                    types.insert(SpanKey::new(span), target.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: target,
                        casts,
                        types,
                    })
                }
                "darcy.vec/is-empty" => {
                    if targs.len() != 1 {
                        return Err(
                            Diag::new("'is-empty' expects 1 argument").with_span(span.clone())
                        );
                    }
                    let _ = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.math/min" | "darcy.math/max" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new("'min/max' expects 2 arguments").with_span(span.clone())
                        );
                    }
                    ctx.unify(&targs[0].ty, &targs[1].ty, &args[1].span())?;
                    let resolved = ctx.resolve(&targs[0].ty);
                    match &resolved {
                        InferTy::Var(_) => {
                            ctx.numeric_constraints
                                .push((resolved.clone(), args[0].span().clone()));
                        }
                        _ => {
                            let concrete = infer_to_ty(ctx, &resolved).ok_or_else(|| {
                                Diag::new("cannot infer min/max argument type")
                                    .with_span(args[0].span().clone())
                            })?;
                            if !is_numeric(&concrete) {
                                return Err(Diag::new("min/max expects numeric type")
                                    .with_span(args[0].span().clone()));
                            }
                        }
                    }
                    types.insert(SpanKey::new(span), resolved.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: resolved,
                        casts,
                        types,
                    })
                }
                "darcy.math/clamp" => {
                    if targs.len() != 3 {
                        return Err(
                            Diag::new("'clamp' expects 3 arguments").with_span(span.clone())
                        );
                    }
                    ctx.unify(&targs[0].ty, &targs[1].ty, &args[1].span())?;
                    ctx.unify(&targs[0].ty, &targs[2].ty, &args[2].span())?;
                    let resolved = ctx.resolve(&targs[0].ty);
                    match &resolved {
                        InferTy::Var(_) => {
                            ctx.numeric_constraints
                                .push((resolved.clone(), args[0].span().clone()));
                        }
                        _ => {
                            let concrete = infer_to_ty(ctx, &resolved).ok_or_else(|| {
                                Diag::new("cannot infer clamp argument type")
                                    .with_span(args[0].span().clone())
                            })?;
                            if !is_numeric(&concrete) {
                                return Err(Diag::new("clamp expects numeric type")
                                    .with_span(args[0].span().clone()));
                            }
                        }
                    }
                    types.insert(SpanKey::new(span), resolved.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: resolved,
                        casts,
                        types,
                    })
                }
                "darcy.math/abs" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'abs' expects 1 argument").with_span(span.clone()));
                    }
                    let arg_ty = targs[0].ty.clone();
                    let resolved = ctx.resolve(&arg_ty);
                    match &resolved {
                        InferTy::Var(_) => {
                            ctx.numeric_constraints
                                .push((resolved.clone(), args[0].span().clone()));
                        }
                        _ => {
                            let concrete = infer_to_ty(ctx, &resolved).ok_or_else(|| {
                                Diag::new("cannot infer abs argument type")
                                    .with_span(args[0].span().clone())
                            })?;
                            if !is_numeric(&concrete) {
                                return Err(Diag::new("abs expects numeric type")
                                    .with_span(args[0].span().clone()));
                            }
                        }
                    }
                    types.insert(SpanKey::new(span), arg_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: arg_ty,
                        casts,
                        types,
                    })
                }
                "darcy.op/gt"
                | "darcy.op/lt"
                | "darcy.op/gte"
                | "darcy.op/lte"
                | "darcy.op/eq"
                | "="
                | "<"
                | ">"
                | "<="
                | ">=" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'cmp' expects 2 arguments").with_span(span.clone()));
                    }
                    ctx.unify(&targs[0].ty, &targs[1].ty, &args[1].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }

                "&" | "|" | "darcy.op/bit-and" | "darcy.op/bit-or" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new("'bitwise' expects 2 arguments").with_span(span.clone())
                        );
                    }
                    let a = &targs[0];
                    let b = &targs[1];
                    let (out_ty, extra_casts) = infer_integer_binop(
                        ctx,
                        &a.ty,
                        &b.ty,
                        &a.expr.span(),
                        &b.expr.span(),
                        span,
                    )?;
                    casts.extend(extra_casts);
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.core/clone" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'clone' expects 1 argument").with_span(span.clone()));
                    }
                    let out_ty = targs[0].ty.clone();
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/map" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'map' expects 2 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[1].ty, &args[1].span())?;
                    let out_elem = ctx.fresh_var();
                    let fn_ty = InferTy::Fn(vec![elem], Box::new(out_elem.clone()));
                    ctx.unify(&targs[0].ty, &fn_ty, &args[0].span())?;
                    let out_ty = InferTy::Vec(Box::new(out_elem));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/len" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'len' expects 1 argument").with_span(span.clone()));
                    }
                    let _ = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("usize".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/new" => {
                    if !targs.is_empty() {
                        return Err(Diag::new("'new' expects 0 arguments").with_span(span.clone()));
                    }
                    let elem = ctx.fresh_var();
                    let out_ty = InferTy::Vec(Box::new(elem));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/get" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'get' expects 2 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let _ = ensure_int_arg(ctx, &targs[1].ty, &args[1].span())?;
                    types.insert(SpanKey::new(span), elem.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: elem,
                        casts,
                        types,
                    })
                }
                "darcy.vec/set" => {
                    if targs.len() != 3 {
                        return Err(Diag::new("'set' expects 3 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let _ = ensure_int_arg(ctx, &targs[1].ty, &args[1].span())?;
                    ctx.unify(&targs[2].ty, &elem, &args[2].span())?;
                    let out_ty = InferTy::Vec(Box::new(elem));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/push" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'push' expects 2 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    ctx.unify(&targs[1].ty, &elem, &args[1].span())?;
                    let out_ty = InferTy::Vec(Box::new(elem));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/repeat" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new("'repeat' expects 2 arguments").with_span(span.clone())
                        );
                    }
                    let elem = targs[0].ty.clone();
                    let _ = ensure_int_arg(ctx, &targs[1].ty, &args[1].span())?;
                    let out_ty = InferTy::Vec(Box::new(elem));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/take" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'take' expects 2 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let _ = ensure_int_arg(ctx, &targs[1].ty, &args[1].span())?;
                    let out_ty = InferTy::Vec(Box::new(elem));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/range" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'range' expects 1 argument").with_span(span.clone()));
                    }
                    let _ = ensure_int_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Vec(Box::new(InferTy::Named("i64".to_string())));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.vec/fold" => {
                    if targs.len() != 3 {
                        return Err(Diag::new("'fold' expects 3 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[2].ty, &args[2].span())?;
                    let acc = targs[1].ty.clone();
                    let fn_ty = InferTy::Fn(vec![acc.clone(), elem], Box::new(acc.clone()));
                    ctx.unify(&targs[0].ty, &fn_ty, &args[0].span())?;
                    types.insert(SpanKey::new(span), acc.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: acc,
                        casts,
                        types,
                    })
                }
                "darcy.vec/map2" => {
                    if targs.len() != 3 {
                        return Err(Diag::new("'map2' expects 3 arguments").with_span(span.clone()));
                    }
                    let left = ensure_vec_arg(ctx, &targs[1].ty, &args[1].span())?;
                    let right = ensure_vec_arg(ctx, &targs[2].ty, &args[2].span())?;
                    let out_elem = ctx.fresh_var();
                    let fn_ty = InferTy::Fn(vec![left, right], Box::new(out_elem.clone()));
                    ctx.unify(&targs[0].ty, &fn_ty, &args[0].span())?;
                    let out_ty = InferTy::Vec(Box::new(out_elem));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.string/split" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new("'split' expects 2 arguments").with_span(span.clone())
                        );
                    }
                    let str_ty = InferTy::Named("string".to_string());
                    ctx.unify(&targs[0].ty, &str_ty, &args[0].span())?;
                    ctx.unify(&targs[1].ty, &str_ty, &args[1].span())?;
                    let out_ty = InferTy::Vec(Box::new(str_ty));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.string/join" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'join' expects 2 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let str_ty = InferTy::Named("string".to_string());
                    ctx.unify(&elem, &str_ty, &args[0].span())?;
                    ctx.unify(&targs[1].ty, &str_ty, &args[1].span())?;
                    let out_ty = InferTy::Named("string".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.string/len" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'len' expects 1 argument").with_span(span.clone()));
                    }
                    let target = InferTy::Named("string".to_string());
                    ctx.unify(&targs[0].ty, &target, &args[0].span())?;
                    let out_ty = InferTy::Named("usize".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.string/is-empty" => {
                    if targs.len() != 1 {
                        return Err(
                            Diag::new("'is-empty' expects 1 argument").with_span(span.clone())
                        );
                    }
                    let target = InferTy::Named("string".to_string());
                    ctx.unify(&targs[0].ty, &target, &args[0].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }

                "darcy.option/unwrap" => {
                    if targs.len() != 1 {
                        return Err(
                            Diag::new("'unwrap' expects 1 argument").with_span(span.clone())
                        );
                    }
                    // Option<T> unwrap returns T
                    let out_ty = ctx.fresh_var();
                    let target = InferTy::Option(Box::new(out_ty.clone()));
                    ctx.unify(&targs[0].ty, &target, &args[0].span())?;

                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.option/unwrap-or" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new("'unwrap-or' expects 2 arguments").with_span(span.clone())
                        );
                    }
                    // Option<T> unwrap-or(fallback: T) returns T
                    let fallback_ty = targs[1].ty.clone();
                    let target = InferTy::Option(Box::new(fallback_ty.clone()));
                    ctx.unify(&targs[0].ty, &target, &args[0].span())?;

                    types.insert(SpanKey::new(span), fallback_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: fallback_ty,
                        casts,
                        types,
                    })
                }
                "darcy.string/trim" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'trim' expects 1 argument").with_span(span.clone()));
                    }
                    let target = InferTy::Named("string".to_string());
                    ctx.unify(&targs[0].ty, &target, &args[0].span())?;
                    let out_ty = InferTy::Named("string".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }

                "darcy.result/unwrap" => {
                    if targs.len() != 1 {
                        return Err(
                            Diag::new("'unwrap' expects 1 argument").with_span(span.clone())
                        );
                    }
                    // Result<T, E> unwrap returns T
                    let out_ty = ctx.fresh_var();
                    let err_ty = ctx.fresh_var();
                    let target = InferTy::Result(Box::new(out_ty.clone()), Box::new(err_ty));
                    ctx.unify(&targs[0].ty, &target, &args[0].span())?;

                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.result/unwrap-or" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new("'unwrap-or' expects 2 arguments").with_span(span.clone())
                        );
                    }
                    // Result<T, E> unwrap-or(fallback: T) returns T
                    let fallback_ty = targs[1].ty.clone();
                    let err_ty = ctx.fresh_var();
                    let target = InferTy::Result(Box::new(fallback_ty.clone()), Box::new(err_ty));
                    ctx.unify(&targs[0].ty, &target, &args[0].span())?;

                    types.insert(SpanKey::new(span), fallback_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: fallback_ty,
                        casts,
                        types,
                    })
                }
                "darcy.hash-map/len" | "darcy.btree-map/len" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'len' expects 1 argument").with_span(span.clone()));
                    }
                    let kind = if op.starts_with("darcy.hash-map/") {
                        MapKind::Hash
                    } else {
                        MapKind::BTree
                    };
                    let _ = ensure_map_arg(ctx, &targs[0].ty, kind, &args[0].span())?;
                    let out_ty = InferTy::Named("usize".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.hash-map/is-empty" | "darcy.btree-map/is-empty" => {
                    if targs.len() != 1 {
                        return Err(
                            Diag::new("'is-empty' expects 1 argument").with_span(span.clone())
                        );
                    }
                    let kind = if op.starts_with("darcy.hash-map/") {
                        MapKind::Hash
                    } else {
                        MapKind::BTree
                    };
                    let _ = ensure_map_arg(ctx, &targs[0].ty, kind, &args[0].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.hash-map/get" | "darcy.btree-map/get" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'get' expects 2 arguments").with_span(span.clone()));
                    }
                    let kind = if op.starts_with("darcy.hash-map/") {
                        MapKind::Hash
                    } else {
                        MapKind::BTree
                    };
                    let (k, v) = ensure_map_arg(ctx, &targs[0].ty, kind, &args[0].span())?;
                    ctx.unify(&k, &targs[1].ty, &args[1].span())?;
                    let out_ty = InferTy::Option(Box::new(v));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.hash-map/contains" | "darcy.btree-map/contains" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new("'contains' expects 2 arguments").with_span(span.clone())
                        );
                    }
                    let kind = if op.starts_with("darcy.hash-map/") {
                        MapKind::Hash
                    } else {
                        MapKind::BTree
                    };
                    let (k, _v) = ensure_map_arg(ctx, &targs[0].ty, kind, &args[0].span())?;
                    ctx.unify(&k, &targs[1].ty, &args[1].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.hash-map/insert" | "darcy.btree-map/insert" => {
                    if targs.len() != 3 {
                        return Err(
                            Diag::new("'insert' expects 3 arguments").with_span(span.clone())
                        );
                    }
                    let kind = if op.starts_with("darcy.hash-map/") {
                        MapKind::Hash
                    } else {
                        MapKind::BTree
                    };
                    let (k, v) = ensure_map_arg(ctx, &targs[0].ty, kind, &args[0].span())?;
                    ctx.unify(&k, &targs[1].ty, &args[1].span())?;
                    ctx.unify(&v, &targs[2].ty, &args[2].span())?;
                    let out_ty = InferTy::Map(kind, Box::new(k), Box::new(v));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "darcy.hash-map/remove" | "darcy.btree-map/remove" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new("'remove' expects 2 arguments").with_span(span.clone())
                        );
                    }
                    let kind = if op.starts_with("darcy.hash-map/") {
                        MapKind::Hash
                    } else {
                        MapKind::BTree
                    };
                    let (k, v) = ensure_map_arg(ctx, &targs[0].ty, kind, &args[0].span())?;
                    ctx.unify(&k, &targs[1].ty, &args[1].span())?;
                    let out_ty = InferTy::Map(kind, Box::new(k), Box::new(v));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                _ if env.structs.contains_key(op) => {
                    let sd = env.structs.get(op).unwrap();
                    let sinfo = field_env.structs.get(op).ok_or_else(|| {
                        Diag::new("internal error: missing struct inference")
                            .with_span(span.clone())
                    })?;
                    if targs.len() != sinfo.fields.len() {
                        return Err(Diag::new(format!(
                            "struct '{}' expects {} fields",
                            op,
                            sd.fields.len()
                        ))
                        .with_span(span.clone()));
                    }
                    for (idx, (arg, field)) in targs.iter().zip(sinfo.fields.iter()).enumerate() {
                        ctx.unify(&arg.ty, &field.ty, &args[idx].span())?;
                    }
                    let out_ty = InferTy::Named(op.to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                _ if env.variants.contains_key(op) => {
                    let (union_name, vdef) = env.variants.get(op).unwrap();
                    let vinf = field_env.variants.get(op).ok_or_else(|| {
                        Diag::new("internal error: missing variant inference")
                            .with_span(span.clone())
                    })?;
                    if targs.len() != vinf.fields.len() {
                        return Err(Diag::new(format!(
                            "variant '{}' expects {} arguments",
                            op,
                            vdef.fields.len()
                        ))
                        .with_span(span.clone()));
                    }
                    for (idx, (arg, field)) in targs.iter().zip(vinf.fields.iter()).enumerate() {
                        ctx.unify(&arg.ty, &field.ty, &args[idx].span())?;
                    }
                    let out_ty = InferTy::Named(union_name.clone());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                _ if fns.get_arity(op, targs.len()).is_some() => {
                    let base_sig = fns.get_arity(op, targs.len()).unwrap();
                    let sig = if current_fn == Some(op.as_str()) {
                        base_sig.clone()
                    } else {
                        instantiate_sig(ctx, base_sig)
                    };
                    for (idx, (arg, param_ty)) in targs.iter().zip(sig.params.iter()).enumerate() {
                        ctx.unify(&arg.ty, param_ty, &args[idx].span())?;
                    }
                    let out_ty = sig.ret.clone();
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                _ if fns.get(op).is_some() => {
                    let supported: Vec<String> = fns
                        .get(op)
                        .unwrap()
                        .iter()
                        .map(|sig| sig.params.len().to_string())
                        .collect();
                    Err(Diag::new(format!(
                        "function '{}' has no overload with arity {}; available arities: {}",
                        op,
                        targs.len(),
                        supported.join(", ")
                    ))
                    .with_span(span.clone()))
                }
                "+" | "-" | "*" | "/" | "darcy.op/add" | "darcy.op/sub" | "darcy.op/mul"
                | "darcy.op/div" => {
                    if targs.len() != 2 {
                        return Err(Diag::new(format!("'{}' expects 2 arguments", op))
                            .with_span(span.clone()));
                    }
                    let a = &targs[0];
                    let b = &targs[1];
                    let (out_ty, extra_casts) = infer_numeric_binop(
                        ctx,
                        &a.ty,
                        &b.ty,
                        &a.expr.span(),
                        &b.expr.span(),
                        span,
                    )?;
                    casts.extend(extra_casts);
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "mod" | "darcy.op/mod" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'mod' expects 2 arguments").with_span(span.clone()));
                    }
                    let a = &targs[0];
                    let b = &targs[1];
                    let (out_ty, extra_casts) = infer_integer_binop(
                        ctx,
                        &a.ty,
                        &b.ty,
                        &a.expr.span(),
                        &b.expr.span(),
                        span,
                    )?;
                    casts.extend(extra_casts);
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                _ => Err(Diag::new(format!("unknown operator '{}'", op)).with_span(span.clone())),
            }
        }
    }
}

fn finalize_infer_expr(ctx: &InferCtx, expr: InferExpr) -> DslResult<TypedExpr> {
    let ty = infer_to_ty(ctx, &expr.ty)
        .ok_or_else(|| Diag::new("cannot infer expression type").with_span(expr.expr.span()))?;
    let mut types = BTreeMap::new();
    for (k, v) in expr.types {
        let resolved =
            infer_to_ty(ctx, &v).ok_or_else(|| Diag::new("cannot infer expression type"))?;
        types.insert(k, resolved);
    }
    Ok(TypedExpr {
        expr: expr.expr,
        ty,
        casts: expr.casts,
        types,
    })
}

fn finalize_infer_expr_allow_generic(ctx: &InferCtx, expr: InferExpr) -> DslResult<TypedExpr> {
    let ty = infer_to_ty_allow_generic(ctx, &expr.ty);
    let mut types = BTreeMap::new();
    for (k, v) in expr.types {
        let resolved = infer_to_ty_allow_generic(ctx, &v);
        types.insert(k, resolved);
    }
    Ok(TypedExpr {
        expr: expr.expr,
        ty,
        casts: expr.casts,
        types,
    })
}

fn ensure_vec_arg(ctx: &mut InferCtx, ty: &InferTy, span: &Span) -> DslResult<InferTy> {
    let resolved = ctx.resolve(ty);
    match resolved {
        InferTy::Vec(inner) => Ok(*inner),
        InferTy::Var(_) => {
            let elem = ctx.fresh_var();
            let vec_ty = InferTy::Vec(Box::new(elem.clone()));
            ctx.unify(&resolved, &vec_ty, span)?;
            Ok(elem)
        }
        _ => Err(Diag::new("expected vector type").with_span(span.clone())),
    }
}

fn ensure_int_arg(ctx: &mut InferCtx, ty: &InferTy, span: &Span) -> DslResult<InferTy> {
    let resolved = ctx.resolve(ty);
    match &resolved {
        InferTy::Var(_) => {
            ctx.int_constraints.push((resolved.clone(), span.clone()));
            Ok(resolved)
        }
        InferTy::Named(name) => {
            if is_integer_name(name) {
                Ok(resolved)
            } else {
                Err(Diag::new("index expects integer type").with_span(span.clone()))
            }
        }
        _ => Err(Diag::new("index expects integer type").with_span(span.clone())),
    }
}

fn ensure_option_arg(ctx: &mut InferCtx, ty: &InferTy, span: &Span) -> DslResult<InferTy> {
    let resolved = ctx.resolve(ty);
    match resolved {
        InferTy::Option(inner) => Ok(*inner),
        InferTy::Var(_) => {
            let inner = ctx.fresh_var();
            let opt = InferTy::Option(Box::new(inner.clone()));
            ctx.unify(&resolved, &opt, span)?;
            Ok(inner)
        }
        _ => Err(Diag::new("expected option type").with_span(span.clone())),
    }
}

fn ensure_result_arg(
    ctx: &mut InferCtx,
    ty: &InferTy,
    span: &Span,
) -> DslResult<(InferTy, InferTy)> {
    let resolved = ctx.resolve(ty);
    match resolved {
        InferTy::Result(ok, err) => Ok((*ok, *err)),
        InferTy::Var(_) => {
            let ok = ctx.fresh_var();
            let err = ctx.fresh_var();
            let res = InferTy::Result(Box::new(ok.clone()), Box::new(err.clone()));
            ctx.unify(&resolved, &res, span)?;
            Ok((ok, err))
        }
        _ => Err(Diag::new("expected result type").with_span(span.clone())),
    }
}

fn ensure_map_arg(
    ctx: &mut InferCtx,
    ty: &InferTy,
    kind: MapKind,
    span: &Span,
) -> DslResult<(InferTy, InferTy)> {
    let resolved = ctx.resolve(ty);
    match resolved {
        InferTy::Map(k, kty, vty) => {
            if k != kind {
                return Err(Diag::new("map kind mismatch").with_span(span.clone()));
            }
            Ok((*kty, *vty))
        }
        InferTy::Var(_) => {
            let k = ctx.fresh_var();
            let v = ctx.fresh_var();
            let map = InferTy::Map(kind, Box::new(k.clone()), Box::new(v.clone()));
            ctx.unify(&resolved, &map, span)?;
            Ok((k, v))
        }
        _ => Err(Diag::new("expected map type").with_span(span.clone())),
    }
}

fn infer_numeric_binop(
    ctx: &mut InferCtx,
    a: &InferTy,
    b: &InferTy,
    a_sp: &Span,
    b_sp: &Span,
    op_sp: &Span,
) -> DslResult<(InferTy, Vec<CastHint>)> {
    let mut a_res = ctx.resolve(a);
    let mut b_res = ctx.resolve(b);

    match (&a_res, &b_res) {
        (InferTy::Var(_), InferTy::Var(_)) => {
            ctx.unify(&a_res, &b_res, op_sp)?;
            let merged = ctx.resolve(&a_res);
            ctx.numeric_constraints
                .push((merged.clone(), op_sp.clone()));
            return Ok((merged, vec![]));
        }
        (InferTy::Var(_), InferTy::Vec(inner)) => {
            ctx.unify(&a_res, inner, op_sp)?;
        }
        (InferTy::Vec(inner), InferTy::Var(_)) => {
            ctx.unify(&b_res, inner, op_sp)?;
        }
        (InferTy::Var(_), _) => {
            ctx.unify(&a_res, &b_res, op_sp)?;
        }
        (_, InferTy::Var(_)) => {
            ctx.unify(&b_res, &a_res, op_sp)?;
        }
        _ => {}
    }

    a_res = ctx.resolve(&a_res);
    b_res = ctx.resolve(&b_res);

    if matches!(a_res, InferTy::Var(_)) || matches!(b_res, InferTy::Var(_)) {
        let vec_ty = match (&a_res, &b_res) {
            (InferTy::Vec(inner), _) => Some(InferTy::Vec(inner.clone())),
            (_, InferTy::Vec(inner)) => Some(InferTy::Vec(inner.clone())),
            _ => None,
        };
        if let Some(vec_ty) = vec_ty {
            if let InferTy::Vec(inner) = &vec_ty {
                let inner_res = ctx.resolve(inner);
                if matches!(inner_res, InferTy::Var(_)) {
                    ctx.numeric_constraints
                        .push((inner_res.clone(), op_sp.clone()));
                }
            }
            return Ok((vec_ty, vec![]));
        }
        let merged = if matches!(a_res, InferTy::Var(_)) {
            a_res.clone()
        } else {
            b_res.clone()
        };
        ctx.numeric_constraints
            .push((merged.clone(), op_sp.clone()));
        return Ok((merged, vec![]));
    }

    let a_ty = infer_to_ty(ctx, &a_res).ok_or_else(|| {
        Diag::new(format!(
            "ambiguous numeric operator types: '{}'",
            infer_ty_rust(ctx, &a_res)
        ))
        .with_span(op_sp.clone())
    })?;
    let b_ty = infer_to_ty(ctx, &b_res).ok_or_else(|| {
        Diag::new(format!(
            "ambiguous numeric operator types: '{}'",
            infer_ty_rust(ctx, &b_res)
        ))
        .with_span(op_sp.clone())
    })?;

    let (out_ty, casts) = numeric_binop(&a_ty, &b_ty, a_sp, b_sp)
        .map_err(|m| Diag::new(m).with_span(op_sp.clone()))?;
    Ok((infer_from_ty(ctx, &out_ty), casts))
}

fn infer_integer_binop(
    ctx: &mut InferCtx,
    a: &InferTy,
    b: &InferTy,
    a_sp: &Span,
    b_sp: &Span,
    op_sp: &Span,
) -> DslResult<(InferTy, Vec<CastHint>)> {
    let mut a_res = ctx.resolve(a);
    let mut b_res = ctx.resolve(b);

    match (&a_res, &b_res) {
        (InferTy::Var(_), InferTy::Var(_)) => {
            ctx.unify(&a_res, &b_res, op_sp)?;
            let merged = ctx.resolve(&a_res);
            ctx.numeric_constraints
                .push((merged.clone(), op_sp.clone()));
            return Ok((merged, vec![]));
        }
        (InferTy::Var(_), InferTy::Vec(inner)) => {
            ctx.unify(&a_res, inner, op_sp)?;
        }
        (InferTy::Vec(inner), InferTy::Var(_)) => {
            ctx.unify(&b_res, inner, op_sp)?;
        }
        (InferTy::Var(_), _) => {
            ctx.unify(&a_res, &b_res, op_sp)?;
        }
        (_, InferTy::Var(_)) => {
            ctx.unify(&b_res, &a_res, op_sp)?;
        }
        _ => {}
    }

    a_res = ctx.resolve(&a_res);
    b_res = ctx.resolve(&b_res);

    if matches!(a_res, InferTy::Var(_)) || matches!(b_res, InferTy::Var(_)) {
        let vec_ty = match (&a_res, &b_res) {
            (InferTy::Vec(inner), _) => Some(InferTy::Vec(inner.clone())),
            (_, InferTy::Vec(inner)) => Some(InferTy::Vec(inner.clone())),
            _ => None,
        };
        if let Some(vec_ty) = vec_ty {
            if let InferTy::Vec(inner) = &vec_ty {
                let inner_res = ctx.resolve(inner);
                if matches!(inner_res, InferTy::Var(_)) {
                    ctx.numeric_constraints
                        .push((inner_res.clone(), op_sp.clone()));
                }
            }
            return Ok((vec_ty, vec![]));
        }
        let merged = if matches!(a_res, InferTy::Var(_)) {
            a_res.clone()
        } else {
            b_res.clone()
        };
        ctx.numeric_constraints
            .push((merged.clone(), op_sp.clone()));
        return Ok((merged, vec![]));
    }

    let a_ty = infer_to_ty(ctx, &a_res).ok_or_else(|| {
        Diag::new(format!(
            "ambiguous integer operator types: '{}'",
            infer_ty_rust(ctx, &a_res)
        ))
        .with_span(op_sp.clone())
    })?;
    let b_ty = infer_to_ty(ctx, &b_res).ok_or_else(|| {
        Diag::new(format!(
            "ambiguous integer operator types: '{}'",
            infer_ty_rust(ctx, &b_res)
        ))
        .with_span(op_sp.clone())
    })?;

    let (out_ty, casts) = integer_binop(&a_ty, &b_ty, a_sp, b_sp)
        .map_err(|m| Diag::new(m).with_span(op_sp.clone()))?;
    Ok((infer_from_ty(ctx, &out_ty), casts))
}

fn numeric_binop(a: &Ty, b: &Ty, a_sp: &Span, b_sp: &Span) -> Result<(Ty, Vec<CastHint>), String> {
    let (vec_side, vec_elem, scalar) = match (a, b) {
        (Ty::Vec(inner), other) => (Some(true), inner.as_ref(), other),
        (other, Ty::Vec(inner)) => (Some(false), inner.as_ref(), other),
        _ => (None, &Ty::Unknown, &Ty::Unknown),
    };

    if let Some(_vec_left) = vec_side {
        if let Ty::Vec(_) = scalar {
            return Err("vector-vector numeric ops are not supported".to_string());
        }
        let elem_ty = vec_elem;
        if elem_ty != scalar {
            return Err(format!(
                "vector-scalar numeric ops require matching element type, got '{}' and '{}'",
                elem_ty.rust(),
                scalar.rust()
            ));
        }
        let out_elem = elem_ty.clone();
        let out_vec = Ty::Vec(Box::new(out_elem));
        return Ok((out_vec, vec![]));
    }

    let out = numeric_scalar_binop(a, b)?;
    let mut casts = Vec::new();
    if out.rust() != a.rust() && is_numeric(a) {
        casts.push(CastHint {
            span: a_sp.clone(),
            target: out.clone(),
        });
    }
    if out.rust() != b.rust() && is_numeric(b) {
        casts.push(CastHint {
            span: b_sp.clone(),
            target: out.clone(),
        });
    }
    Ok((out, casts))
}

fn integer_binop(a: &Ty, b: &Ty, a_sp: &Span, b_sp: &Span) -> Result<(Ty, Vec<CastHint>), String> {
    let (vec_side, vec_elem, scalar) = match (a, b) {
        (Ty::Vec(inner), other) => (Some(true), inner.as_ref(), other),
        (other, Ty::Vec(inner)) => (Some(false), inner.as_ref(), other),
        _ => (None, &Ty::Unknown, &Ty::Unknown),
    };

    if let Some(_vec_left) = vec_side {
        if let Ty::Vec(_) = scalar {
            return Err("vector-vector mod is not supported".to_string());
        }
        if !is_integer(vec_elem) || !is_integer(scalar) {
            return Err("mod expects integer types".to_string());
        }
        if vec_elem != scalar {
            return Err(format!(
                "vector-scalar mod requires matching element type, got '{}' and '{}'",
                vec_elem.rust(),
                scalar.rust()
            ));
        }
        let out_vec = Ty::Vec(Box::new(vec_elem.clone()));
        return Ok((out_vec, vec![]));
    }

    if !is_integer(a) || !is_integer(b) {
        return Err(format!(
            "mod expects integer types, got '{}' and '{}'",
            a.rust(),
            b.rust()
        ));
    }

    let out = if a == b {
        a.clone()
    } else {
        Ty::Named("i64".to_string())
    };
    let mut casts = Vec::new();
    if out.rust() != a.rust() {
        casts.push(CastHint {
            span: a_sp.clone(),
            target: out.clone(),
        });
    }
    if out.rust() != b.rust() {
        casts.push(CastHint {
            span: b_sp.clone(),
            target: out.clone(),
        });
    }
    Ok((out, casts))
}

fn numeric_scalar_binop(a: &Ty, b: &Ty) -> Result<Ty, String> {
    let ai = matches!(a, Ty::Named(n) if is_integer_name(n));
    let af = matches!(a, Ty::Named(n) if n == "f32" || n == "f64");
    let bi = matches!(b, Ty::Named(n) if is_integer_name(n));
    let bf = matches!(b, Ty::Named(n) if n == "f32" || n == "f64");

    if !(ai || af) || !(bi || bf) {
        return Err(format!(
            "operator expects numeric types, got '{}' and '{}'",
            a.rust(),
            b.rust()
        ));
    }

    if a == &Ty::Named("f64".to_string()) || b == &Ty::Named("f64".to_string()) {
        return Ok(Ty::Named("f64".to_string()));
    }

    if af || bf {
        return Ok(Ty::Named("f32".to_string()));
    }

    if a == b {
        return Ok(a.clone());
    }

    Ok(Ty::Named("i64".to_string()))
}

fn is_numeric_name(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "isize"
            | "f32"
            | "f64"
    )
}

fn is_numeric(t: &Ty) -> bool {
    if let Ty::Named(name) = t {
        is_numeric_name(name)
    } else {
        false
    }
}

fn is_integer_name(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "isize"
    )
}

fn is_integer(t: &Ty) -> bool {
    matches!(t, Ty::Named(s) if is_integer_name(s))
}

fn is_cast_target(ty: &Ty) -> bool {
    matches!(ty, Ty::Named(s) if is_numeric_name(s))
}

fn is_cast_source_name(name: &str) -> bool {
    is_numeric_name(name) || name == "bool"
}

fn check_numeric_constraints(ctx: &mut InferCtx, keep_vars: &BTreeSet<u32>) -> DslResult<()> {
    for (ty, span) in &ctx.numeric_constraints {
        let resolved = ctx.resolve(ty);
        if let InferTy::Var(id) = resolved {
            if keep_vars.contains(&id) {
                continue;
            }
            ctx.subs.insert(id, InferTy::Named("i64".to_string()));
            continue;
        }
        let concrete = infer_to_ty(ctx, &resolved).ok_or_else(|| {
            Diag::new("ambiguous numeric operator types; add a literal or annotation")
                .with_span(span.clone())
        })?;
        if !is_numeric(&concrete) {
            return Err(Diag::new(format!(
                "operator expects numeric type, got '{}'",
                concrete.rust()
            ))
            .with_span(span.clone()));
        }
    }
    Ok(())
}

fn check_int_constraints(ctx: &mut InferCtx, keep_vars: &BTreeSet<u32>) -> DslResult<()> {
    for (ty, span) in &ctx.int_constraints {
        let resolved = ctx.resolve(ty);
        if let InferTy::Var(id) = resolved {
            if keep_vars.contains(&id) {
                continue;
            }
            ctx.subs.insert(id, InferTy::Named("i64".to_string()));
            continue;
        }
        if let InferTy::Named(name) = &resolved {
            if is_integer_name(name) {
                continue;
            }
        }
        return Err(Diag::new("index expects integer type").with_span(span.clone()));
    }
    Ok(())
}

fn inline_subst_iterable(
    iter: &crate::ast::Iterable,
    map: &BTreeMap<String, Expr>,
) -> crate::ast::Iterable {
    match iter {
        crate::ast::Iterable::Range(r) => crate::ast::Iterable::Range(inline_subst_range(map, r)),
        crate::ast::Iterable::Expr(e) => {
            crate::ast::Iterable::Expr(Box::new(inline_subst_local(e, map)))
        }
    }
}

fn inline_subst_iterable_local(
    iter: &crate::ast::Iterable,
    inline_defs: &BTreeMap<String, crate::ast::InlineDef>,
) -> DslResult<crate::ast::Iterable> {
    match iter {
        crate::ast::Iterable::Range(r) => Ok(crate::ast::Iterable::Range(
            inline_subst_range_local(r, inline_defs)?,
        )),
        crate::ast::Iterable::Expr(e) => Ok(crate::ast::Iterable::Expr(Box::new(
            expand_inline_calls(e, inline_defs)?,
        ))),
    }
}

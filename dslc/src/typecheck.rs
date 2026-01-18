use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Def, Expr, FnDef, MapKind, MatchArm, MatchPat, StructDef, Top, Ty, UnionDef, VariantDef,
};
use crate::diag::{Diag, DslResult, Span};
use crate::typed::{CastHint, SpanKey, TypedDef, TypedExpr, TypedFn};

#[derive(Debug, Clone)]
pub struct FnSig {
    pub params: Vec<Ty>,
    pub ret: Ty,
}

#[derive(Debug, Clone)]
pub struct FnEnv {
    pub fns: BTreeMap<String, FnSig>,
}

impl FnEnv {
    pub fn new() -> Self {
        Self {
            fns: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, sig: FnSig) -> DslResult<()> {
        if self.fns.contains_key(&name) {
            return Err(Diag::new(format!("duplicate function '{}'", name)));
        }
        self.fns.insert(name, sig);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&FnSig> {
        self.fns.get(name)
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
                    Diag::new(format!("duplicate variant '{}'", v.name)).with_span(v.span.clone()),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InferTy {
    Var(u32),
    Named(String),
    Vec(Box<InferTy>),
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
}

impl InferCtx {
    fn new() -> Self {
        Self {
            next_var: 0,
            subs: BTreeMap::new(),
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
            InferTy::Option(inner) => InferTy::Option(Box::new(self.resolve(inner))),
            InferTy::Result(ok, err) => {
                InferTy::Result(Box::new(self.resolve(ok)), Box::new(self.resolve(err)))
            }
            InferTy::Map(kind, k, v) => {
                InferTy::Map(kind.clone(), Box::new(self.resolve(k)), Box::new(self.resolve(v)))
            }
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
            (InferTy::Option(a), InferTy::Option(b)) => self.unify(&a, &b, span),
            (InferTy::Result(a_ok, a_err), InferTy::Result(b_ok, b_err)) => {
                self.unify(&a_ok, &b_ok, span)?;
                self.unify(&a_err, &b_err, span)
            }
            (InferTy::Map(ka, a_k, a_v), InferTy::Map(kb, b_k, b_v)) => {
                if ka != kb {
                    return Err(Diag::new("type mismatch: map kind differs").with_span(span.clone()));
                }
                self.unify(&a_k, &b_k, span)?;
                self.unify(&a_v, &b_v, span)
            }
            (InferTy::Fn(a_params, a_ret), InferTy::Fn(b_params, b_ret)) => {
                if a_params.len() != b_params.len() {
                    return Err(Diag::new("type mismatch: function arity differs")
                        .with_span(span.clone()));
                }
                for (a, b) in a_params.iter().zip(b_params.iter()) {
                    self.unify(a, b, span)?;
                }
                self.unify(&a_ret, &b_ret, span)
            }
            (InferTy::Vec(_), InferTy::Named(_)) | (InferTy::Named(_), InferTy::Vec(_)) => {
                Err(Diag::new("type mismatch: vector vs scalar").with_span(span.clone()))
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
    match ty {
        Ty::Named(n) => InferTy::Named(n.clone()),
        Ty::Vec(inner) => InferTy::Vec(Box::new(infer_from_ty(ctx, inner))),
        Ty::Option(inner) => InferTy::Option(Box::new(infer_from_ty(ctx, inner))),
        Ty::Result(ok, err) => InferTy::Result(
            Box::new(infer_from_ty(ctx, ok)),
            Box::new(infer_from_ty(ctx, err)),
        ),
        Ty::Map(kind, k, v) => InferTy::Map(
            kind.clone(),
            Box::new(infer_from_ty(ctx, k)),
            Box::new(infer_from_ty(ctx, v)),
        ),
        Ty::Unknown => ctx.fresh_var(),
    }
}

fn infer_to_ty(ctx: &InferCtx, ty: &InferTy) -> Option<Ty> {
    match ctx.resolve(ty) {
        InferTy::Var(_) => None,
        InferTy::Named(n) => Some(Ty::Named(n)),
        InferTy::Vec(inner) => infer_to_ty(ctx, &inner).map(|t| Ty::Vec(Box::new(t))),
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

fn infer_ty_rust(ctx: &InferCtx, ty: &InferTy) -> String {
    match ctx.resolve(ty) {
        InferTy::Var(id) => format!("'t{}", id),
        InferTy::Named(n) => n,
        InferTy::Vec(inner) => format!("Vec<{}>", infer_ty_rust(ctx, &inner)),
        InferTy::Option(inner) => format!("Option<{}>", infer_ty_rust(ctx, &inner)),
        InferTy::Result(ok, err) => {
            format!("Result<{}, {}>", infer_ty_rust(ctx, &ok), infer_ty_rust(ctx, &err))
        }
        InferTy::Map(kind, k, v) => {
            let name = match kind {
                MapKind::Hash => "HashMap",
                MapKind::BTree => "BTreeMap",
            };
            format!("{}<{}, {}>", name, infer_ty_rust(ctx, &k), infer_ty_rust(ctx, &v))
        }
        InferTy::Fn(params, ret) => {
            let args = params.iter().map(|p| infer_ty_rust(ctx, p)).collect::<Vec<_>>();
            format!("fn({}) -> {}", args.join(", "), infer_ty_rust(ctx, &ret))
        }
    }
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

    let mut typed_fns = Vec::new();
    let mut typed_defs = Vec::new();
    let mut fn_env = FnEnv::new();
    let def_base_names = collect_def_base_names(&filtered);
    let mut global_defs: BTreeMap<String, Ty> = BTreeMap::new();
    for t in &filtered {
        match t {
            Top::Func(fd) => {
                if env.structs.contains_key(&fd.name)
                    || env.unions.contains_key(&fd.name)
                    || env.variants.contains_key(&fd.name)
                    || fn_env.get(&fd.name).is_some()
                    || global_defs.contains_key(&fd.name)
                {
                    return Err(Diag::new(format!("duplicate function '{}'", fd.name))
                        .with_span(fd.span.clone()));
                }
                let typed = typecheck_fn(&env, &fn_env, &global_defs, &def_base_names, fd)?;
                let sig = FnSig {
                    params: fd
                        .params
                        .iter()
                        .map(|p| typed.param_tys[&p.rust_name].clone())
                        .collect(),
                    ret: typed.body.ty.clone(),
                };
                fn_env.insert(fd.name.clone(), sig)?;
                typed_fns.push(typed);
            }
            Top::Def(d) => {
                if env.structs.contains_key(&d.name)
                    || env.unions.contains_key(&d.name)
                    || env.variants.contains_key(&d.name)
                    || fn_env.get(&d.name).is_some()
                    || global_defs.contains_key(&d.name)
                {
                    return Err(Diag::new(format!("duplicate def '{}'", d.name))
                        .with_span(d.span.clone()));
                }
                let typed = typecheck_def(&env, &fn_env, &global_defs, &def_base_names, &d)?;
                global_defs.insert(d.name.clone(), typed.body.ty.clone());
                typed_defs.push(typed);
            }
            _ => {}
        }
    }

    Ok(TypecheckedProgram {
        env,
        typed_fns,
        typed_defs,
    })
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
    name.replace('-', "_")
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
                    return Err(
                        Diag::new(format!(
                            "inline '{}' expects {} arguments",
                            inl.name,
                            inl.params.len()
                        ))
                        .with_span(span.clone()),
                    );
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
        Expr::If { cond, then_br, else_br, span } => Ok(Expr::If {
            cond: Box::new(expand_inline_calls(cond, inline_defs)?),
            then_br: Box::new(expand_inline_calls(then_br, inline_defs)?),
            else_br: match else_br {
                Some(b) => Some(Box::new(expand_inline_calls(b, inline_defs)?)),
                None => None,
            },
            span: span.clone(),
        }),
        Expr::Let { bindings, body, span } => {
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
        Expr::For { var, iter, body, span } => Ok(Expr::For {
            var: var.clone(),
            iter: inline_subst_iterable_local(iter, inline_defs)?,
            body: Box::new(expand_inline_calls(body, inline_defs)?),
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
        Expr::Match { scrutinee, arms, span } => {
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
        Expr::MapLit { kind, entries, span, ann } => {
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
        Expr::Int(..)
        | Expr::Float(..)
        | Expr::Str(..)
        | Expr::Var(..)
        | Expr::Continue { .. } => Ok(expr.clone()),
    }
}

fn inline_subst_local(expr: &Expr, map: &BTreeMap<String, Expr>) -> Expr {
    match expr {
        Expr::Var(name, _) => map.get(name).cloned().unwrap_or_else(|| expr.clone()),
        Expr::Int(..) | Expr::Float(..) | Expr::Str(..) | Expr::Continue { .. } => expr.clone(),
        Expr::Pair { key, val, span } => Expr::Pair {
            key: Box::new(inline_subst_local(key, map)),
            val: Box::new(inline_subst_local(val, map)),
            span: span.clone(),
        },
        Expr::Let { bindings, body, span } => {
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
        Expr::If { cond, then_br, else_br, span } => Expr::If {
            cond: Box::new(inline_subst_local(cond, map)),
            then_br: Box::new(inline_subst_local(then_br, map)),
            else_br: else_br.as_ref().map(|b| Box::new(inline_subst_local(b, map))),
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
        Expr::For { var, iter, body, span } => Expr::For {
            var: var.clone(),
            iter: inline_subst_iterable(iter, map),
            body: Box::new(inline_subst_local(body, map)),
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
        Expr::Match { scrutinee, arms, span } => {
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
        Expr::VecLit { elems, span, ann } => Expr::VecLit {
            elems: elems.iter().map(|e| inline_subst_local(e, map)).collect(),
            span: span.clone(),
            ann: ann.clone(),
        },
        Expr::MapLit { kind, entries, span, ann } => {
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
    }
}

fn inline_subst_range(
    map: &BTreeMap<String, Expr>,
    range: &crate::ast::RangeExpr,
) -> crate::ast::RangeExpr {
    crate::ast::RangeExpr {
        start: Box::new(inline_subst_local(&range.start, map)),
        end: Box::new(inline_subst_local(&range.end, map)),
        step: range.step.as_ref().map(|s| Box::new(inline_subst_local(s, map))),
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

pub fn typecheck_fn(
    env: &TypeEnv,
    fns: &FnEnv,
    global_defs: &BTreeMap<String, Ty>,
    def_base_names: &BTreeSet<String>,
    f: &FnDef,
) -> DslResult<TypedFn> {
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
        let ret = f
            .extern_ret
            .clone()
            .ok_or_else(|| Diag::new("extern function must declare return type").with_span(f.span.clone()))?;
        let body = TypedExpr {
            expr: f.body.clone(),
            ty: ret,
            casts: vec![],
            types: BTreeMap::new(),
        };
        return Ok(TypedFn {
            def: f.clone(),
            param_tys,
            body,
        });
    }

    let mut ctx = InferCtx::new();
    let (param_tys, param_spans) = infer_param_types(env, f, def_base_names, &mut ctx)?;
    let mut vars = BTreeMap::new();
    for (k, v) in &param_tys {
        vars.insert(k.clone(), v.clone());
    }

    let infer_body = infer_expr_type(env, fns, global_defs, def_base_names, &mut ctx, &vars, &f.body)?;
    let mut final_param_tys = BTreeMap::new();
    for (name, ty) in &param_tys {
        let resolved = infer_to_ty(&ctx, ty).ok_or_else(|| {
            Diag::new(format!(
                "cannot infer type for parameter '{}': no constraints. Add annotation like {}:Type",
                name, name
            ))
            .with_span(param_spans[name].clone())
        })?;
        final_param_tys.insert(name.clone(), resolved);
    }

    let body = finalize_infer_expr(&ctx, infer_body).map_err(|mut d| {
        if d.span.is_none() {
            d = d.with_span(f.body.span());
        }
        d
    })?;

    Ok(TypedFn {
        def: f.clone(),
        param_tys: final_param_tys,
        body,
    })
}

fn typecheck_def(
    env: &TypeEnv,
    fns: &FnEnv,
    global_defs: &BTreeMap<String, Ty>,
    def_base_names: &BTreeSet<String>,
    d: &Def,
) -> DslResult<TypedDef> {
    let mut ctx = InferCtx::new();
    let vars: BTreeMap<String, InferTy> = BTreeMap::new();
    let mut infer_body =
        infer_expr_type(env, fns, global_defs, def_base_names, &mut ctx, &vars, &d.expr)?;
    if let Some(ann) = &d.ann {
        let ann_ty = infer_from_ty(&mut ctx, ann);
        ctx.unify(&ann_ty, &infer_body.ty, &d.span)?;
        infer_body.ty = ann_ty;
    }
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

fn infer_param_types(
    env: &TypeEnv,
    f: &FnDef,
    def_base_names: &BTreeSet<String>,
    ctx: &mut InferCtx,
) -> DslResult<(BTreeMap<String, InferTy>, BTreeMap<String, Span>)> {
    let mut param_tys: BTreeMap<String, InferTy> = BTreeMap::new();
    let mut param_spans: BTreeMap<String, Span> = BTreeMap::new();

    for p in &f.params {
        if param_tys.contains_key(&p.rust_name) {
            return Err(
                Diag::new(format!("duplicate parameter '{}'", p.name)).with_span(p.span.clone()),
            );
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
        param_tys.insert(p.rust_name.clone(), ty);
        param_spans.insert(p.rust_name.clone(), p.span.clone());
    }

    let mut constraints: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut seen = Vec::new();
    collect_field_constraints(&f.body, &mut seen);
    for (p, field) in seen {
        constraints.entry(p).or_default().insert(field);
    }

    for (p, fields) in constraints {
        if !param_tys.contains_key(&p) {
            return Err(
                Diag::new(format!("unknown variable '{}'", p)).with_span(f.body.span()),
            );
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
            InferTy::Option(_) | InferTy::Result(_, _) | InferTy::Map(_, _, _) | InferTy::Fn(_, _) => {
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
    Ok((param_tys, param_spans))
}

fn fmt_set(s: &BTreeSet<String>) -> String {
    let mut v: Vec<_> = s.iter().cloned().collect();
    v.sort();
    format!("{{{}}}", v.join(", "))
}

#[derive(Debug, Clone)]
struct LoopFrame {
    result_ty: InferTy,
    saw_break: bool,
}

fn infer_expr_type(
    env: &TypeEnv,
    fns: &FnEnv,
    globals: &BTreeMap<String, Ty>,
    def_base_names: &BTreeSet<String>,
    ctx: &mut InferCtx,
    vars: &BTreeMap<String, InferTy>,
    e: &Expr,
) -> DslResult<InferExpr> {
    let mut loop_stack = Vec::new();
    infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, &mut loop_stack, e)
}

fn infer_expr_type_internal(
    env: &TypeEnv,
    fns: &FnEnv,
    globals: &BTreeMap<String, Ty>,
    def_base_names: &BTreeSet<String>,
    ctx: &mut InferCtx,
    vars: &BTreeMap<String, InferTy>,
    loop_stack: &mut Vec<LoopFrame>,
    e: &Expr,
) -> DslResult<InferExpr> {
    match e {
        Expr::Int(_, sp) => {
            let ty = InferTy::Named("i32".to_string());
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
        Expr::Var(v, sp) => {
            let ty = if let Some(local) = vars.get(v) {
                local.clone()
            } else if let Some(global) = globals.get(v) {
                infer_from_ty(ctx, global)
            } else {
                return Err(
                    Diag::new(format!("unknown variable '{}'", v)).with_span(sp.clone())
                );
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
        Expr::VecLit { elems, span, ann } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut elem_tys = Vec::new();
            for el in elems {
                let te = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, el)?;
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
        Expr::Let { bindings, body, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut local_vars = vars.clone();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for b in bindings {
                if !seen.insert(b.rust_name.clone()) {
                    return Err(
                        Diag::new(format!("duplicate let binding '{}'", b.name))
                            .with_span(b.span.clone()),
                    );
                }
                if def_base_names.contains(&b.rust_name) {
                    return Err(
                        Diag::new(format!("let binding '{}' shadows a def name", b.name))
                            .with_span(b.span.clone()),
                    );
                }
                let te = infer_expr_type_internal(
                    env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    &local_vars,
                    loop_stack,
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
                fns,
                globals,
                def_base_names,
                ctx,
                &local_vars,
                loop_stack,
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
        Expr::Lambda { params, body, span: _ } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut param_tys = Vec::new();
            let mut local_vars = vars.clone();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for p in params {
                if !seen.insert(p.rust_name.clone()) {
                    return Err(
                        Diag::new(format!("duplicate parameter '{}'", p.name))
                            .with_span(p.span.clone()),
                    );
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
                fns,
                globals,
                def_base_names,
                ctx,
                &local_vars,
                loop_stack,
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
                fns,
                globals,
                def_base_names,
                ctx,
                vars,
                loop_stack,
                func,
            )?;
            casts.extend(tfunc.casts.clone());
            types.extend(tfunc.types);
            let mut arg_tys = Vec::new();
            for a in args {
                let ta = infer_expr_type_internal(
                    env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    vars,
                    loop_stack,
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
        Expr::Do { exprs, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let mut last_ty = InferTy::Named("()".to_string());
            for ex in exprs {
                let te = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, ex)?;
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
            let tcond = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, cond)?;
            casts.extend(tcond.casts.clone());
            types.extend(tcond.types.clone());
            let bool_ty = InferTy::Named("bool".to_string());
            ctx.unify(&tcond.ty, &bool_ty, &cond.span())?;

            let tthen = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, then_br)?;
            casts.extend(tthen.casts.clone());
            types.extend(tthen.types.clone());

            let out_ty = if let Some(else_br) = else_br {
                let telse = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, else_br)?;
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
            let tbody = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, body)?;
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
            let tcond = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, cond)?;
            casts.extend(tcond.casts.clone());
            types.extend(tcond.types.clone());
            let bool_ty = InferTy::Named("bool".to_string());
            ctx.unify(&tcond.ty, &bool_ty, &cond.span())?;

            let result_ty = ctx.fresh_var();
            loop_stack.push(LoopFrame {
                result_ty: result_ty.clone(),
                saw_break: false,
            });
            let tbody = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, body)?;
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
        Expr::For { var, iter, body, span } => {
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();

            let elem_ty = match iter {
                crate::ast::Iterable::Range(range) => {
                    let tstart = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, &range.start)?;
                    let tend = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, &range.end)?;
                    casts.extend(tstart.casts.clone());
                    casts.extend(tend.casts.clone());
                    types.extend(tstart.types.clone());
                    types.extend(tend.types.clone());
                    ctx.unify(&tstart.ty, &tend.ty, &range.end.span())?;
                    let mut elem_ty = ctx.resolve(&tstart.ty);
                    if let Some(step) = &range.step {
                        let tstep = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, step)?;
                        casts.extend(tstep.casts.clone());
                        types.extend(tstep.types.clone());
                        ctx.unify(&elem_ty, &tstep.ty, &step.span())?;
                        elem_ty = ctx.resolve(&elem_ty);
                    }
                    let elem_resolved = infer_to_ty(ctx, &elem_ty)
                        .ok_or_else(|| Diag::new("cannot infer range element type").with_span(range.span.clone()))?;
                    if !is_numeric(&elem_resolved) {
                        return Err(Diag::new("range bounds must be numeric").with_span(range.span.clone()));
                    }
                    elem_ty
                }
                crate::ast::Iterable::Expr(ex) => {
                    let titer = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, ex)?;
                    casts.extend(titer.casts.clone());
                    types.extend(titer.types.clone());
                    let iter_ty = ctx.resolve(&titer.ty);
                    match iter_ty {
                        InferTy::Vec(inner) => *inner,
                        InferTy::Var(_) => {
                            let inner = ctx.fresh_var();
                            ctx.unify(&titer.ty, &InferTy::Vec(Box::new(inner.clone())), &ex.span())?;
                            inner
                        }
                        _ => {
                            return Err(Diag::new("for loop iterable must be a vector").with_span(ex.span()));
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
                fns,
                globals,
                def_base_names,
                ctx,
                &body_vars,
                loop_stack,
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
        Expr::Break { value, span } => {
            if loop_stack.is_empty() {
                return Err(
                    Diag::new("break is only allowed inside loops").with_span(span.clone())
                );
            }
            let frame_idx = loop_stack.len() - 1;
            let result_ty = loop_stack[frame_idx].result_ty.clone();
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            let value_ty = if let Some(v) = value {
                let tv = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, v)?;
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
        Expr::Pair { span, .. } => {
            Err(Diag::new("pair literal is only allowed inside hashmap/new or btreemap/new")
                .with_span(span.clone()))
        }
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
                    "cannot infer map type from empty literal; add hashmap<K,V> annotation",
                )
                .with_span(span.clone()));
            }

            if let Some((k, v)) = ann {
                key_ty = Some(infer_from_ty(ctx, k));
                val_ty = Some(infer_from_ty(ctx, v));
            }

            for (k, v) in entries {
                let tk = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, k)?;
                let tv = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, v)?;
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
            let tb = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, base)?;
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
                },
                InferTy::Var(_) => {
                    return Err(
                        Diag::new("cannot access field on unknown type").with_span(span.clone()),
                    )
                }
                InferTy::Option(_) | InferTy::Result(_, _) | InferTy::Map(_, _, _) | InferTy::Fn(_, _) => {
                    return Err(
                        Diag::new("cannot access field on non-struct type").with_span(span.clone()),
                    )
                }
            };
            let sd = env
                .structs
                .get(&struct_name)
                .ok_or_else(|| Diag::new(format!("unknown type '{}'", struct_name)).with_span(span.clone()))?;
            let f = sd
                .fields
                .iter()
                .find(|ff| ff.rust_name == *field)
                .ok_or_else(|| {
                    Diag::new(format!("type '{}' has no field '{}'", struct_name, field))
                        .with_span(span.clone())
                })?;

            let field_ty = infer_from_ty(ctx, &f.ty);
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
        Expr::Match { scrutinee, arms, span } => {
            let scrut = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, scrutinee)?;
            let union_name = match ctx.resolve(&scrut.ty) {
                InferTy::Named(n) => n,
                InferTy::Var(_) => {
                    return Err(
                        Diag::new("cannot match on unknown type").with_span(span.clone()),
                    )
                }
                InferTy::Vec(_) => {
                    return Err(Diag::new("cannot match on vector type").with_span(span.clone()))
                }
                InferTy::Option(_) | InferTy::Result(_, _) | InferTy::Map(_, _, _) | InferTy::Fn(_, _) => {
                    return Err(Diag::new("cannot match on non-union type").with_span(span.clone()))
                }
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
                    MatchPat::Variant { name, bindings, span: psp } => {
                        let (u, vdef) = env
                            .variants
                            .get(name)
                            .ok_or_else(|| Diag::new(format!("unknown variant '{}'", name)).with_span(psp.clone()))?;
                        if u != &union_name {
                            return Err(Diag::new(format!(
                                "variant '{}' does not belong to '{}'",
                                name, union_name
                            ))
                            .with_span(psp.clone()));
                        }
                        seen.insert(name.clone());
                        for (field, binding, bspan) in bindings {
                            let f = vdef
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
                                arm_vars.insert(binding.clone(), infer_from_ty(ctx, &f.ty));
                            }
                        }
                    }
                    MatchPat::Wildcard(_) => {
                        has_wildcard = true;
                    }
                }

                let tarm = infer_expr_type_internal(
                    env,
                    fns,
                    globals,
                    def_base_names,
                    ctx,
                    &arm_vars,
                    loop_stack,
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

            if !has_wildcard {
                let all = &env.unions.get(&union_name).unwrap().variants;
                if seen.len() != all.len() {
                    let mut missing = Vec::new();
                    for v in all {
                        if !seen.contains(&v.name) {
                            missing.push(v.name.clone());
                        }
                    }
                    return Err(Diag::new(format!(
                        "non-exhaustive match, missing: {}",
                        missing.join(", ")
                    ))
                    .with_span(span.clone()));
                }
            }

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
            if op == "core.hashmap/new" || op == "core.btreemap/new" {
                let kind = if op == "core.hashmap/new" {
                    MapKind::Hash
                } else {
                    MapKind::BTree
                };
                if args.is_empty() {
                    return Err(Diag::new(
                        "cannot infer map type from empty literal; add hashmap<K,V> annotation",
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
                        _ => {
                            return Err(
                                Diag::new("map entry must be (key value)").with_span(entry.span())
                            )
                        }
                    };
                    let tk = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, key)?;
                    let tv = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, val)?;
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
                let ta = infer_expr_type_internal(env, fns, globals, def_base_names, ctx, vars, loop_stack, a)?;
                casts.extend(ta.casts.clone());
                types.extend(ta.types.clone());
                targs.push(ta);
            }

            match op.as_str() {
                "dbg" | "std.io/dbg" => {
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
                "core.fmt/dbg" => {
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
                "core.fmt/format" | "core.fmt/pretty" => {
                    if targs.len() != 1 {
                        return Err(
                            Diag::new("format/pretty expects 1 argument").with_span(span.clone())
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
                "core.fmt/print" | "core.fmt/println" => {
                    if targs.is_empty() {
                        return Err(
                            Diag::new("print/println expects a format string").with_span(span.clone())
                        );
                    }
                    ensure_fmt_string(&targs[0].expr, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("()".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.num/abs" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'abs' expects 1 argument").with_span(span.clone()));
                    }
                    let out_ty = infer_numeric_unary(ctx, &targs[0].ty, &args[0].span(), span)?;
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.num/min" | "core.num/max" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("min/max expects 2 arguments").with_span(span.clone()));
                    }
                    let (out_ty, extra_casts) =
                        infer_numeric_minmax(ctx, &targs[0].ty, &targs[1].ty, &args[0].span(), &args[1].span(), span)?;
                    casts.extend(extra_casts);
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.num/clamp" => {
                    if targs.len() != 3 {
                        return Err(Diag::new("'clamp' expects 3 arguments").with_span(span.clone()));
                    }
                    let (out_ty, extra_casts) =
                        infer_numeric_clamp(ctx, &targs[0].ty, &targs[1].ty, &targs[2].ty, &args[0].span(), &args[1].span(), &args[2].span(), span)?;
                    casts.extend(extra_casts);
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.vec/len" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'len' expects 1 argument").with_span(span.clone()));
                    }
                    let _elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("usize".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.vec/is-empty" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'is-empty' expects 1 argument").with_span(span.clone()));
                    }
                    let _elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.vec/get" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'get' expects 2 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    ensure_index_arg(ctx, &targs[1].ty, &args[1].span())?;
                    let out_ty = elem;
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.vec/set" => {
                    if targs.len() != 3 {
                        return Err(Diag::new("'set' expects 3 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    ensure_index_arg(ctx, &targs[1].ty, &args[1].span())?;
                    ctx.unify(&elem, &targs[2].ty, &args[2].span())?;
                    let out_ty = InferTy::Named("()".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.str/len" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'len' expects 1 argument").with_span(span.clone()));
                    }
                    ensure_string_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("usize".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.str/is-empty" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'is-empty' expects 1 argument").with_span(span.clone()));
                    }
                    ensure_string_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("bool".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.str/trim" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'trim' expects 1 argument").with_span(span.clone()));
                    }
                    ensure_string_arg(ctx, &targs[0].ty, &args[0].span())?;
                    let out_ty = InferTy::Named("string".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.str/split" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'split' expects 2 arguments").with_span(span.clone()));
                    }
                    ensure_string_arg(ctx, &targs[0].ty, &args[0].span())?;
                    ensure_string_arg(ctx, &targs[1].ty, &args[1].span())?;
                    let out_ty = InferTy::Vec(Box::new(InferTy::Named("string".to_string())));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.str/join" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'join' expects 2 arguments").with_span(span.clone()));
                    }
                    let elem = ensure_vec_arg(ctx, &targs[0].ty, &args[0].span())?;
                    ensure_string_arg(ctx, &elem, &args[0].span())?;
                    ensure_string_arg(ctx, &targs[1].ty, &args[1].span())?;
                    let out_ty = InferTy::Named("string".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.option/some" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'some' expects 1 argument").with_span(span.clone()));
                    }
                    let inner = targs[0].ty.clone();
                    let out_ty = InferTy::Option(Box::new(inner));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.option/none" => {
                    if !targs.is_empty() {
                        return Err(Diag::new("'none' expects 0 arguments").with_span(span.clone()));
                    }
                    let out_ty = InferTy::Option(Box::new(ctx.fresh_var()));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.option/is-some" | "core.option/is-none" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("option predicate expects 1 argument").with_span(span.clone()));
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
                "core.option/unwrap" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'unwrap' expects 1 argument").with_span(span.clone()));
                    }
                    let inner = ensure_option_arg(ctx, &targs[0].ty, &args[0].span())?;
                    types.insert(SpanKey::new(span), inner.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: inner,
                        casts,
                        types,
                    })
                }
                "core.option/unwrap-or" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'unwrap-or' expects 2 arguments").with_span(span.clone()));
                    }
                    let inner = ensure_option_arg(ctx, &targs[0].ty, &args[0].span())?;
                    ctx.unify(&inner, &targs[1].ty, &args[1].span())?;
                    types.insert(SpanKey::new(span), inner.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: inner,
                        casts,
                        types,
                    })
                }
                "core.result/ok" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'ok' expects 1 argument").with_span(span.clone()));
                    }
                    let ok = targs[0].ty.clone();
                    let err = ctx.fresh_var();
                    let out_ty = InferTy::Result(Box::new(ok), Box::new(err));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.result/err" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'err' expects 1 argument").with_span(span.clone()));
                    }
                    let err = targs[0].ty.clone();
                    let ok = ctx.fresh_var();
                    let out_ty = InferTy::Result(Box::new(ok), Box::new(err));
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "core.result/is-ok" | "core.result/is-err" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("result predicate expects 1 argument").with_span(span.clone()));
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
                "core.result/unwrap" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'unwrap' expects 1 argument").with_span(span.clone()));
                    }
                    let (ok, _err) = ensure_result_arg(ctx, &targs[0].ty, &args[0].span())?;
                    types.insert(SpanKey::new(span), ok.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: ok,
                        casts,
                        types,
                    })
                }
                "core.result/unwrap-or" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'unwrap-or' expects 2 arguments").with_span(span.clone()));
                    }
                    let (ok, _err) = ensure_result_arg(ctx, &targs[0].ty, &args[0].span())?;
                    ctx.unify(&ok, &targs[1].ty, &args[1].span())?;
                    types.insert(SpanKey::new(span), ok.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: ok,
                        casts,
                        types,
                    })
                }
                "core.hashmap/len" | "core.btreemap/len" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'len' expects 1 argument").with_span(span.clone()));
                    }
                    let kind = if op.starts_with("core.hashmap/") {
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
                "core.hashmap/is-empty" | "core.btreemap/is-empty" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'is-empty' expects 1 argument").with_span(span.clone()));
                    }
                    let kind = if op.starts_with("core.hashmap/") {
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
                "core.hashmap/get" | "core.btreemap/get" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'get' expects 2 arguments").with_span(span.clone()));
                    }
                    let kind = if op.starts_with("core.hashmap/") {
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
                "core.hashmap/contains" | "core.btreemap/contains" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'contains' expects 2 arguments").with_span(span.clone()));
                    }
                    let kind = if op.starts_with("core.hashmap/") {
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
                "core.hashmap/insert" | "core.btreemap/insert" => {
                    if targs.len() != 3 {
                        return Err(Diag::new("'insert' expects 3 arguments").with_span(span.clone()));
                    }
                    let kind = if op.starts_with("core.hashmap/") {
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
                "core.hashmap/remove" | "core.btreemap/remove" => {
                    if targs.len() != 2 {
                        return Err(Diag::new("'remove' expects 2 arguments").with_span(span.clone()));
                    }
                    let kind = if op.starts_with("core.hashmap/") {
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
                    if targs.len() != sd.fields.len() {
                        return Err(Diag::new(format!(
                            "struct '{}' expects {} fields",
                            op,
                            sd.fields.len()
                        ))
                        .with_span(span.clone()));
                    }
                    for (idx, (arg, field)) in targs.iter().zip(sd.fields.iter()).enumerate() {
                        let field_ty = infer_from_ty(ctx, &field.ty);
                        ctx.unify(&arg.ty, &field_ty, &args[idx].span())?;
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
                    if targs.len() != vdef.fields.len() {
                        return Err(Diag::new(format!(
                            "variant '{}' expects {} arguments",
                            op,
                            vdef.fields.len()
                        ))
                        .with_span(span.clone()));
                    }
                    for (idx, (arg, field)) in targs.iter().zip(vdef.fields.iter()).enumerate() {
                        let field_ty = infer_from_ty(ctx, &field.ty);
                        ctx.unify(&arg.ty, &field_ty, &args[idx].span())?;
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
                _ if fns.get(op).is_some() => {
                    let sig = fns.get(op).unwrap();
                    if targs.len() != sig.params.len() {
                        return Err(Diag::new(format!(
                            "function '{}' expects {} arguments",
                            op,
                            sig.params.len()
                        ))
                        .with_span(span.clone()));
                    }
                    for (idx, (arg, param_ty)) in targs.iter().zip(sig.params.iter()).enumerate() {
                        let param_ty = infer_from_ty(ctx, param_ty);
                        ctx.unify(&arg.ty, &param_ty, &args[idx].span())?;
                    }
                    let out_ty = infer_from_ty(ctx, &sig.ret);
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(InferExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                        types,
                    })
                }
                "+" | "-" | "*" | "/" => {
                    if targs.len() != 2 {
                        return Err(
                            Diag::new(format!("'{}' expects 2 arguments", op)).with_span(span.clone()),
                        );
                    }
                    let a = &targs[0];
                    let b = &targs[1];
                    let (out_ty, extra_casts) =
                        infer_numeric_binop(ctx, &a.ty, &b.ty, &a.expr.span(), &b.expr.span(), span)?;
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

fn ensure_fmt_string(expr: &Expr, ty: &InferTy, span: &Span) -> DslResult<()> {
    match expr {
        Expr::Str(_, _) => Ok(()),
        _ => match ty {
            InferTy::Named(n) if n == "string" => Err(
                Diag::new("format string must be a string literal").with_span(span.clone()),
            ),
            _ => Err(Diag::new("format string must be a string literal").with_span(span.clone())),
        },
    }
}

fn finalize_infer_expr(ctx: &InferCtx, expr: InferExpr) -> DslResult<TypedExpr> {
    let ty = infer_to_ty(ctx, &expr.ty).ok_or_else(|| {
        Diag::new("cannot infer expression type").with_span(expr.expr.span())
    })?;
    let mut types = BTreeMap::new();
    for (k, v) in expr.types {
        let resolved = infer_to_ty(ctx, &v)
            .ok_or_else(|| Diag::new("cannot infer expression type"))?;
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
        InferTy::Named(name) => Err(
            Diag::new(format!("expected vector type, got '{}'", name)).with_span(span.clone()),
        ),
        InferTy::Option(_) | InferTy::Result(_, _) | InferTy::Map(_, _, _) | InferTy::Fn(_, _) => {
            Err(Diag::new("expected vector type").with_span(span.clone()))
        }
    }
}

fn ensure_index_arg(ctx: &mut InferCtx, ty: &InferTy, span: &Span) -> DslResult<()> {
    let resolved = ctx.resolve(ty);
    match resolved {
        InferTy::Named(name) => {
            if is_integer_name(&name) {
                Ok(())
            } else {
                Err(Diag::new("index must be an integer").with_span(span.clone()))
            }
        }
        InferTy::Var(_) => {
            let int_ty = InferTy::Named("i32".to_string());
            ctx.unify(&resolved, &int_ty, span)?;
            Ok(())
        }
        InferTy::Vec(_)
        | InferTy::Option(_)
        | InferTy::Result(_, _)
        | InferTy::Map(_, _, _)
        | InferTy::Fn(_, _) => {
            Err(Diag::new("index must be an integer").with_span(span.clone()))
        }
    }
}

fn is_integer_name(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize"
    )
}

fn ensure_string_arg(ctx: &mut InferCtx, ty: &InferTy, span: &Span) -> DslResult<()> {
    let resolved = ctx.resolve(ty);
    match resolved {
        InferTy::Named(name) if name == "string" => Ok(()),
        InferTy::Var(_) => {
            ctx.unify(&resolved, &InferTy::Named("string".to_string()), span)?;
            Ok(())
        }
        InferTy::Named(name) => Err(
            Diag::new(format!("expected string type, got '{}'", name)).with_span(span.clone()),
        ),
        InferTy::Vec(_) => Err(Diag::new("expected string type, got vector").with_span(span.clone())),
        InferTy::Option(_) | InferTy::Result(_, _) | InferTy::Map(_, _, _) | InferTy::Fn(_, _) => {
            Err(Diag::new("expected string type").with_span(span.clone()))
        }
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

fn ensure_result_arg(ctx: &mut InferCtx, ty: &InferTy, span: &Span) -> DslResult<(InferTy, InferTy)> {
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
fn infer_numeric_unary(
    ctx: &mut InferCtx,
    ty: &InferTy,
    arg_sp: &Span,
    op_sp: &Span,
) -> DslResult<InferTy> {
    let resolved = ctx.resolve(ty);
    match resolved {
        InferTy::Var(_) => Err(Diag::new(
            "ambiguous numeric operator types; add a literal or annotation",
        )
        .with_span(op_sp.clone())),
        InferTy::Vec(_) => Err(Diag::new("numeric operators expect scalars")
            .with_span(arg_sp.clone())),
        InferTy::Option(_) | InferTy::Result(_, _) | InferTy::Map(_, _, _) | InferTy::Fn(_, _) => Err(
            Diag::new("numeric operators expect scalars").with_span(arg_sp.clone()),
        ),
        InferTy::Named(_) => {
            let out = infer_to_ty(ctx, &resolved).ok_or_else(|| {
                Diag::new("ambiguous numeric operator types").with_span(op_sp.clone())
            })?;
            if !is_numeric(&out) {
                return Err(
                    Diag::new(format!("operator expects numeric type, got '{}'", out.rust()))
                        .with_span(arg_sp.clone()),
                );
            }
            Ok(resolved)
        }
    }
}

fn infer_numeric_minmax(
    ctx: &mut InferCtx,
    a: &InferTy,
    b: &InferTy,
    a_sp: &Span,
    b_sp: &Span,
    op_sp: &Span,
) -> DslResult<(InferTy, Vec<CastHint>)> {
    let a_ty = infer_to_ty(ctx, &ctx.resolve(a)).ok_or_else(|| {
        Diag::new("ambiguous numeric operator types; add a literal or annotation")
            .with_span(op_sp.clone())
    })?;
    let b_ty = infer_to_ty(ctx, &ctx.resolve(b)).ok_or_else(|| {
        Diag::new("ambiguous numeric operator types; add a literal or annotation")
            .with_span(op_sp.clone())
    })?;
    if matches!(a_ty, Ty::Vec(_)) || matches!(b_ty, Ty::Vec(_)) {
        return Err(Diag::new("min/max expects scalar numeric arguments").with_span(op_sp.clone()));
    }
    let (out_ty, casts) =
        numeric_binop(&a_ty, &b_ty, a_sp, b_sp).map_err(|m| Diag::new(m).with_span(op_sp.clone()))?;
    Ok((infer_from_ty(ctx, &out_ty), casts))
}

fn infer_numeric_clamp(
    ctx: &mut InferCtx,
    x: &InferTy,
    min: &InferTy,
    max: &InferTy,
    x_sp: &Span,
    min_sp: &Span,
    max_sp: &Span,
    op_sp: &Span,
) -> DslResult<(InferTy, Vec<CastHint>)> {
    let x_ty = infer_to_ty(ctx, &ctx.resolve(x)).ok_or_else(|| {
        Diag::new("ambiguous numeric operator types; add a literal or annotation")
            .with_span(op_sp.clone())
    })?;
    let min_ty = infer_to_ty(ctx, &ctx.resolve(min)).ok_or_else(|| {
        Diag::new("ambiguous numeric operator types; add a literal or annotation")
            .with_span(op_sp.clone())
    })?;
    let max_ty = infer_to_ty(ctx, &ctx.resolve(max)).ok_or_else(|| {
        Diag::new("ambiguous numeric operator types; add a literal or annotation")
            .with_span(op_sp.clone())
    })?;
    if matches!(x_ty, Ty::Vec(_)) || matches!(min_ty, Ty::Vec(_)) || matches!(max_ty, Ty::Vec(_)) {
        return Err(Diag::new("clamp expects scalar numeric arguments").with_span(op_sp.clone()));
    }

    let (out_ty, casts) =
        numeric_binop(&x_ty, &min_ty, x_sp, min_sp).map_err(|m| Diag::new(m).with_span(op_sp.clone()))?;
    let (out_ty2, mut casts2) =
        numeric_binop(&out_ty, &max_ty, x_sp, max_sp).map_err(|m| Diag::new(m).with_span(op_sp.clone()))?;
    let mut all_casts = casts;
    all_casts.append(&mut casts2);
    Ok((infer_from_ty(ctx, &out_ty2), all_casts))
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
            return Err(Diag::new(
                "ambiguous numeric operator types; add a literal or annotation",
            )
            .with_span(op_sp.clone()))
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
        return Err(Diag::new(
            "ambiguous numeric operator types; add a literal or annotation",
        )
        .with_span(op_sp.clone()));
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

    let (out_ty, casts) =
        numeric_binop(&a_ty, &b_ty, a_sp, b_sp).map_err(|m| Diag::new(m).with_span(op_sp.clone()))?;
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

fn numeric_scalar_binop(a: &Ty, b: &Ty) -> Result<Ty, String> {
    let ai = a == &Ty::Named("i32".to_string())
        || a == &Ty::Named("i64".to_string())
        || a == &Ty::Named("u32".to_string())
        || a == &Ty::Named("u64".to_string());
    let af = a == &Ty::Named("f32".to_string()) || a == &Ty::Named("f64".to_string());
    let bi = b == &Ty::Named("i32".to_string())
        || b == &Ty::Named("i64".to_string())
        || b == &Ty::Named("u32".to_string())
        || b == &Ty::Named("u64".to_string());
    let bf = b == &Ty::Named("f32".to_string()) || b == &Ty::Named("f64".to_string());

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

    Ok(Ty::Named("i32".to_string()))
}

fn is_numeric(t: &Ty) -> bool {
    matches!(
        t,
        Ty::Named(s)
        if s == "i32"
            || s == "i64"
            || s == "u32"
            || s == "u64"
            || s == "f32"
            || s == "f64"
    )
}

fn inline_subst_iterable(
    iter: &crate::ast::Iterable,
    map: &BTreeMap<String, Expr>,
) -> crate::ast::Iterable {
    match iter {
        crate::ast::Iterable::Range(r) => crate::ast::Iterable::Range(inline_subst_range(map, r)),
        crate::ast::Iterable::Expr(e) => crate::ast::Iterable::Expr(Box::new(inline_subst_local(e, map))),
    }
}

fn inline_subst_iterable_local(
    iter: &crate::ast::Iterable,
    inline_defs: &BTreeMap<String, crate::ast::InlineDef>,
) -> DslResult<crate::ast::Iterable> {
    match iter {
        crate::ast::Iterable::Range(r) => Ok(crate::ast::Iterable::Range(inline_subst_range_local(r, inline_defs)?)),
        crate::ast::Iterable::Expr(e) => Ok(crate::ast::Iterable::Expr(Box::new(expand_inline_calls(e, inline_defs)?))),
    }
}

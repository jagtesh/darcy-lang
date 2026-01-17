use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, FnDef, MatchPat, StructDef, Top, Ty, UnionDef, VariantDef};
use crate::diag::{Diag, DslResult, Span};
use crate::typed::{CastHint, SpanKey, TypedExpr, TypedFn};

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
}

pub fn typecheck_tops(tops: &[Top]) -> DslResult<TypecheckedProgram> {
    let mut env = TypeEnv::new();
    for t in tops {
        if let Top::Struct(sd) = t {
            env.insert_struct(sd.clone())?;
        }
        if let Top::Union(ud) = t {
            env.insert_union(ud.clone())?;
        }
    }

    let mut typed_fns = Vec::new();
    let mut fn_env = FnEnv::new();
    for t in tops {
        if let Top::Func(fd) = t {
            if env.structs.contains_key(&fd.name)
                || env.unions.contains_key(&fd.name)
                || env.variants.contains_key(&fd.name)
                || fn_env.get(&fd.name).is_some()
            {
                return Err(Diag::new(format!("duplicate function '{}'", fd.name)).with_span(fd.span.clone()));
            }
            let typed = typecheck_fn(&env, &fn_env, fd)?;
            let sig = FnSig {
                params: fd
                    .params
                    .iter()
                    .map(|p| typed.param_tys[&p.name].clone())
                    .collect(),
                ret: typed.body.ty.clone(),
            };
            fn_env.insert(fd.name.clone(), sig)?;
            typed_fns.push(typed);
        }
    }

    Ok(TypecheckedProgram { env, typed_fns })
}

pub fn typecheck_fn(env: &TypeEnv, fns: &FnEnv, f: &FnDef) -> DslResult<TypedFn> {
    let param_tys = infer_param_types(env, f)?;
    let mut vars = BTreeMap::new();
    for (k, v) in &param_tys {
        vars.insert(k.clone(), v.clone());
    }

    let body = infer_expr_type(env, fns, &vars, &f.body)?;
    Ok(TypedFn {
        def: f.clone(),
        param_tys,
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

fn infer_param_types(env: &TypeEnv, f: &FnDef) -> DslResult<BTreeMap<String, Ty>> {
    let mut param_tys: BTreeMap<String, Ty> = BTreeMap::new();
    let mut param_spans: BTreeMap<String, Span> = BTreeMap::new();

    for p in &f.params {
        if param_tys.contains_key(&p.name) {
            return Err(
                Diag::new(format!("duplicate parameter '{}'", p.name)).with_span(p.span.clone()),
            );
        }
        param_tys.insert(p.name.clone(), p.ann.clone().unwrap_or(Ty::Unknown));
        param_spans.insert(p.name.clone(), p.span.clone());
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
        let cur = param_tys.get(&p).cloned().unwrap_or(Ty::Unknown);
        match &cur {
            Ty::Named(ty_name) => {
                let sd = env.structs.get(ty_name).ok_or_else(|| {
                    Diag::new(format!("unknown type '{}'", ty_name))
                        .with_span(param_spans[&p].clone())
                })?;
                for fld in &fields {
                    if !sd.fields.iter().any(|ff| ff.name == *fld) {
                        return Err(Diag::new(format!(
                            "type '{}' has no field '{}'",
                            ty_name, fld
                        ))
                        .with_span(param_spans[&p].clone()));
                    }
                }
                continue;
            }
            Ty::Vec(inner) => {
                let ty_name = match inner.as_ref() {
                    Ty::Named(n) => n.clone(),
                    Ty::Unknown => {
                        return Err(Diag::new("cannot access field on unknown vector type")
                            .with_span(param_spans[&p].clone()))
                    }
                    other => {
                        return Err(Diag::new(format!(
                            "cannot access field on non-struct vector type '{}'",
                            other.rust()
                        ))
                        .with_span(param_spans[&p].clone()))
                    }
                };
                let sd = env.structs.get(&ty_name).ok_or_else(|| {
                    Diag::new(format!("unknown type '{}'", ty_name))
                        .with_span(param_spans[&p].clone())
                })?;
                for fld in &fields {
                    if !sd.fields.iter().any(|ff| ff.name == *fld) {
                        return Err(Diag::new(format!(
                            "type '{}' has no field '{}'",
                            ty_name, fld
                        ))
                        .with_span(param_spans[&p].clone()));
                    }
                }
                continue;
            }
            Ty::Unknown => {}
        }

        let mut candidates = Vec::new();
        for (name, sd) in &env.structs {
            let ok = fields
                .iter()
                .all(|fld| sd.fields.iter().any(|ff| &ff.name == fld));
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
        param_tys.insert(p, Ty::Named(candidates[0].clone()));
    }

    for (p, ty) in param_tys.clone() {
        if ty == Ty::Unknown {
            return Err(Diag::new(format!(
                "cannot infer type for parameter '{}': no constraints. Add annotation like {}:Type",
                p, p
            ))
            .with_span(param_spans[&p].clone()));
        }
    }

    Ok(param_tys)
}

fn fmt_set(s: &BTreeSet<String>) -> String {
    let mut v: Vec<_> = s.iter().cloned().collect();
    v.sort();
    format!("{{{}}}", v.join(", "))
}

fn infer_expr_type(
    env: &TypeEnv,
    fns: &FnEnv,
    vars: &BTreeMap<String, Ty>,
    e: &Expr,
) -> DslResult<TypedExpr> {
    match e {
        Expr::Int(_, sp) => {
            let ty = Ty::Named("i32".to_string());
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(TypedExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Float(_, sp) => {
            let ty = Ty::Named("f64".to_string());
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(TypedExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
                types,
            })
        }
        Expr::Var(v, sp) => {
            let ty = vars
                .get(v)
                .cloned()
                .ok_or_else(|| Diag::new(format!("unknown variable '{}'", v)).with_span(sp.clone()))?;
            let mut types = BTreeMap::new();
            types.insert(SpanKey::new(sp), ty.clone());
            Ok(TypedExpr {
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
                let te = infer_expr_type(env, fns, vars, el)?;
                casts.extend(te.casts.clone());
                types.extend(te.types);
                elem_tys.push((te.ty, el.span()));
            }

            let elem_ty = match ann {
                Some(Ty::Vec(inner)) => {
                    for (ty, el_sp) in &elem_tys {
                        if ty != inner.as_ref() {
                            return Err(Diag::new(format!(
                                "vector element type mismatch: expected '{}', got '{}'",
                                inner.rust(),
                                ty.rust()
                            ))
                            .with_span(el_sp.clone()));
                        }
                    }
                    *inner.clone()
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
                    let (first_ty, _) = &elem_tys[0];
                    for (ty, el_sp) in &elem_tys[1..] {
                        if ty != first_ty {
                            return Err(Diag::new(format!(
                                "vector element type mismatch: expected '{}', got '{}'",
                                first_ty.rust(),
                                ty.rust()
                            ))
                            .with_span(el_sp.clone()));
                        }
                    }
                    first_ty.clone()
                }
            };

            let ty = Ty::Vec(Box::new(elem_ty));
            types.insert(SpanKey::new(span), ty.clone());
            Ok(TypedExpr {
                expr: e.clone(),
                ty,
                casts,
                types,
            })
        }
        Expr::Field { base, field, span } => {
            let tb = infer_expr_type(env, fns, vars, base)?;
            let base_ty = tb.ty.clone();
            let (struct_name, is_vec) = match base_ty {
                Ty::Named(n) => (n, false),
                Ty::Vec(inner) => match *inner {
                    Ty::Named(n) => (n, true),
                    Ty::Unknown => {
                        return Err(Diag::new("cannot access field on unknown type")
                            .with_span(span.clone()))
                    }
                    other => {
                        return Err(Diag::new(format!(
                            "cannot access field on non-struct vector type '{}'",
                            other.rust()
                        ))
                        .with_span(span.clone()))
                    }
                },
                Ty::Unknown => {
                    return Err(
                        Diag::new("cannot access field on unknown type").with_span(span.clone()),
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
                .find(|ff| ff.name == *field)
                .ok_or_else(|| {
                    Diag::new(format!("type '{}' has no field '{}'", struct_name, field))
                        .with_span(span.clone())
                })?;

            let ty = if is_vec {
                Ty::Vec(Box::new(f.ty.clone()))
            } else {
                f.ty.clone()
            };
            let mut types = tb.types;
            types.insert(SpanKey::new(span), ty.clone());
            Ok(TypedExpr {
                expr: e.clone(),
                ty,
                casts: tb.casts,
                types,
            })
        }
        Expr::Match { scrutinee, arms, span } => {
            let scrut = infer_expr_type(env, fns, vars, scrutinee)?;
            let union_name = match &scrut.ty {
                Ty::Named(n) => n.clone(),
                Ty::Unknown => {
                    return Err(
                        Diag::new("cannot match on unknown type").with_span(span.clone()),
                    )
                }
                other => {
                    return Err(Diag::new(format!(
                        "match expects a union type, got '{}'",
                        other.rust()
                    ))
                    .with_span(span.clone()))
                }
            };
            let union = env.unions.get(&union_name).ok_or_else(|| {
                Diag::new(format!("match expects a union type, got '{}'", union_name))
                    .with_span(span.clone())
            })?;

            let mut arm_types = Vec::new();
            let mut casts = scrut.casts.clone();
            let mut types = scrut.types.clone();
            let mut seen_variants = BTreeSet::new();
            let mut has_wildcard = false;

            for arm in arms {
                let mut arm_vars = vars.clone();
                match &arm.pat {
                    MatchPat::Wildcard(_) => {
                        has_wildcard = true;
                    }
                    MatchPat::Variant { name, bindings, span: pspan } => {
                        let (_, vdef) = env.variants.get(name).ok_or_else(|| {
                            Diag::new(format!("unknown variant '{}'", name)).with_span(pspan.clone())
                        })?;
                        let (v_union, _) = env.variants.get(name).unwrap();
                        if v_union != &union_name {
                            return Err(Diag::new(format!(
                                "variant '{}' does not belong to union '{}'",
                                name, union_name
                            ))
                            .with_span(pspan.clone()));
                        }
                        if vdef.fields.len() != bindings.len() {
                            return Err(Diag::new(format!(
                                "variant '{}' expects {} bindings",
                                name,
                                vdef.fields.len()
                            ))
                            .with_span(pspan.clone()));
                        }
                        if seen_variants.contains(name) {
                            return Err(Diag::new(format!(
                                "duplicate match arm for variant '{}'",
                                name
                            ))
                            .with_span(pspan.clone()));
                        }
                        seen_variants.insert(name.clone());
                        let mut seen_fields = BTreeSet::new();
                        for (field_name, bind_name, bspan) in bindings {
                            if seen_fields.contains(field_name) {
                                return Err(Diag::new(format!(
                                    "duplicate field binding '{}'",
                                    field_name
                                ))
                                .with_span(bspan.clone()));
                            }
                            seen_fields.insert(field_name.clone());
                            let fdef = vdef
                                .fields
                                .iter()
                                .find(|f| f.name == *field_name)
                                .ok_or_else(|| {
                                    Diag::new(format!(
                                        "variant '{}' has no field '{}'",
                                        name, field_name
                                    ))
                                    .with_span(bspan.clone())
                                })?;
                            if bind_name != "_" {
                                arm_vars.insert(bind_name.clone(), fdef.ty.clone());
                            }
                        }
                    }
                }
                let tarm = infer_expr_type(env, fns, &arm_vars, &arm.body)?;
                casts.extend(tarm.casts.clone());
                types.extend(tarm.types.clone());
                arm_types.push((tarm.ty, arm.span.clone()));
            }

            if !has_wildcard {
                let mut missing = Vec::new();
                for v in &union.variants {
                    if !seen_variants.contains(&v.name) {
                        missing.push(v.name.clone());
                    }
                }
                if !missing.is_empty() {
                    return Err(Diag::new(format!(
                        "non-exhaustive match: missing {}",
                        missing.join(", ")
                    ))
                    .with_span(span.clone()));
                }
            }

            let (first_ty, _) = arm_types
                .get(0)
                .ok_or_else(|| Diag::new("match requires at least one arm").with_span(span.clone()))?;
            for (ty, tsp) in &arm_types[1..] {
                if ty != first_ty {
                    return Err(Diag::new(format!(
                        "match arms must return the same type; got '{}' and '{}'",
                        first_ty.rust(),
                        ty.rust()
                    ))
                    .with_span(tsp.clone()));
                }
            }
            types.insert(SpanKey::new(span), first_ty.clone());
            Ok(TypedExpr {
                expr: e.clone(),
                ty: first_ty.clone(),
                casts,
                types,
            })
        }
        Expr::Call { op, args, span } => {
            let mut targs = Vec::new();
            let mut casts = Vec::new();
            let mut types = BTreeMap::new();
            for a in args {
                let ta = infer_expr_type(env, fns, vars, a)?;
                casts.extend(ta.casts.clone());
                types.extend(ta.types.clone());
                targs.push(ta);
            }

            match op.as_str() {
                "print" => {
                    if targs.len() != 1 {
                        return Err(Diag::new("'print' expects 1 argument").with_span(span.clone()));
                    }
                    let out_ty = Ty::Named("()".to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(TypedExpr {
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
                        if arg.ty != field.ty {
                            return Err(Diag::new(format!(
                                "struct '{}' field '{}' expects '{}', got '{}'",
                                op,
                                field.name,
                                field.ty.rust(),
                                arg.ty.rust()
                            ))
                            .with_span(args[idx].span()));
                        }
                    }
                    let out_ty = Ty::Named(op.to_string());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(TypedExpr {
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
                        if arg.ty != field.ty {
                            return Err(Diag::new(format!(
                                "variant '{}' field '{}' expects '{}', got '{}'",
                                op,
                                field.name,
                                field.ty.rust(),
                                arg.ty.rust()
                            ))
                            .with_span(args[idx].span()));
                        }
                    }
                    let out_ty = Ty::Named(union_name.clone());
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(TypedExpr {
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
                        if arg.ty != *param_ty {
                            return Err(Diag::new(format!(
                                "function '{}' argument {} expects '{}', got '{}'",
                                op,
                                idx + 1,
                                param_ty.rust(),
                                arg.ty.rust()
                            ))
                            .with_span(args[idx].span()));
                        }
                    }
                    let out_ty = sig.ret.clone();
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(TypedExpr {
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
                        numeric_binop(&a.ty, &b.ty, &a.expr.span(), &b.expr.span())
                            .map_err(|m| Diag::new(m).with_span(span.clone()))?;
                    casts.extend(extra_casts);
                    types.insert(SpanKey::new(span), out_ty.clone());
                    Ok(TypedExpr {
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

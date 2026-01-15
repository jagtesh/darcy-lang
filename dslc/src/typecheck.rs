use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, FnDef, StructDef, Top, Ty};
use crate::diag::{Diag, DslResult, Span};
use crate::typed::{CastHint, TypedExpr, TypedFn};

#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub structs: BTreeMap<String, StructDef>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            structs: BTreeMap::new(),
        }
    }

    pub fn insert_struct(&mut self, sd: StructDef) -> DslResult<()> {
        if self.structs.contains_key(&sd.name) {
            return Err(Diag::new(format!("duplicate struct '{}'", sd.name)).with_span(sd.span));
        }
        self.structs.insert(sd.name.clone(), sd);
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
    }

    let mut typed_fns = Vec::new();
    for t in tops {
        if let Top::Func(fd) = t {
            typed_fns.push(typecheck_fn(&env, fd)?);
        }
    }

    Ok(TypecheckedProgram { env, typed_fns })
}

pub fn typecheck_fn(env: &TypeEnv, f: &FnDef) -> DslResult<TypedFn> {
    let param_tys = infer_param_types(env, f)?;
    let mut vars = BTreeMap::new();
    for (k, v) in &param_tys {
        vars.insert(k.clone(), v.clone());
    }

    let body = infer_expr_type(env, &vars, &f.body)?;
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
        if let Ty::Named(_) = cur {
            let ty_name = match &cur {
                Ty::Named(s) => s.clone(),
                _ => unreachable!(),
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

fn infer_expr_type(env: &TypeEnv, vars: &BTreeMap<String, Ty>, e: &Expr) -> DslResult<TypedExpr> {
    match e {
        Expr::Int(_, _sp) => Ok(TypedExpr {
            expr: e.clone(),
            ty: Ty::Named("i32".to_string()),
            casts: vec![],
        }),
        Expr::Float(_, _sp) => Ok(TypedExpr {
            expr: e.clone(),
            ty: Ty::Named("f64".to_string()),
            casts: vec![],
        }),
        Expr::Var(v, sp) => {
            let ty = vars
                .get(v)
                .cloned()
                .ok_or_else(|| Diag::new(format!("unknown variable '{}'", v)).with_span(sp.clone()))?;
            Ok(TypedExpr {
                expr: e.clone(),
                ty,
                casts: vec![],
            })
        }
        Expr::Field { base, field, span } => {
            let tb = infer_expr_type(env, vars, base)?;
            let base_ty = tb.ty.clone();
            let struct_name = match base_ty {
                Ty::Named(n) => n,
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
            Ok(TypedExpr {
                expr: e.clone(),
                ty: f.ty.clone(),
                casts: tb.casts,
            })
        }
        Expr::Call { op, args, span } => {
            let mut targs = Vec::new();
            let mut casts = Vec::new();
            for a in args {
                let ta = infer_expr_type(env, vars, a)?;
                casts.extend(ta.casts.clone());
                targs.push(ta);
            }

            match op.as_str() {
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
                    Ok(TypedExpr {
                        expr: e.clone(),
                        ty: out_ty,
                        casts,
                    })
                }
                _ => Err(Diag::new(format!("unknown operator '{}'", op)).with_span(span.clone())),
            }
        }
    }
}

fn numeric_binop(a: &Ty, b: &Ty, a_sp: &Span, b_sp: &Span) -> Result<(Ty, Vec<CastHint>), String> {
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
        let mut casts = Vec::new();
        if ai && a != &Ty::Named("f64".to_string()) {
            casts.push(CastHint {
                span: a_sp.clone(),
                target: Ty::Named("f64".to_string()),
            });
        }
        if bi && b != &Ty::Named("f64".to_string()) {
            casts.push(CastHint {
                span: b_sp.clone(),
                target: Ty::Named("f64".to_string()),
            });
        }
        return Ok((Ty::Named("f64".to_string()), casts));
    }

    if af || bf {
        let mut casts = Vec::new();
        if ai && a != &Ty::Named("f32".to_string()) {
            casts.push(CastHint {
                span: a_sp.clone(),
                target: Ty::Named("f32".to_string()),
            });
        }
        if bi && b != &Ty::Named("f32".to_string()) {
            casts.push(CastHint {
                span: b_sp.clone(),
                target: Ty::Named("f32".to_string()),
            });
        }
        return Ok((Ty::Named("f32".to_string()), casts));
    }

    Ok((Ty::Named("i32".to_string()), vec![]))
}

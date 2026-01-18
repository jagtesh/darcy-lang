use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::ast::{Expr, MatchArm, MatchPat, Top, Ty, UseDecl};
use crate::diag::{Diag, DslResult, Span};

#[derive(Debug, Clone)]
struct ModuleInfo {
    name: String,
    uses: Vec<UseDecl>,
    inlines: BTreeMap<String, crate::ast::InlineDef>,
    tops: Vec<Top>,
}

#[derive(Debug, Clone)]
struct ModuleDefs {
    types: BTreeSet<String>,
    variants: BTreeSet<String>,
    fns: BTreeSet<String>,
    inlines: BTreeSet<String>,
    values: BTreeSet<String>,
}

pub fn compile_modules(
    root_path: &Path,
    src: &str,
    lib_paths: &[PathBuf],
) -> DslResult<Vec<Top>> {
    let root_mod = module_name_from_path(root_path)?;
    let mut loader = ModuleLoader::new(lib_paths);
    loader.add_module(&root_mod, src)?;
    loader.resolve_all()?;
    Ok(loader.into_tops(&root_mod)?)
}

struct ModuleLoader {
    lib_paths: Vec<PathBuf>,
    modules: BTreeMap<String, ModuleInfo>,
    defs: BTreeMap<String, ModuleDefs>,
    inline_defs: BTreeMap<String, BTreeMap<String, crate::ast::InlineDef>>,
}

impl ModuleLoader {
    fn new(lib_paths: &[PathBuf]) -> Self {
        Self {
            lib_paths: lib_paths.to_vec(),
            modules: BTreeMap::new(),
            defs: BTreeMap::new(),
            inline_defs: BTreeMap::new(),
        }
    }

    fn add_module(&mut self, name: &str, src: &str) -> DslResult<()> {
        if self.modules.contains_key(name) {
            return Ok(());
        }
        let mut p = crate::Parser::new(crate::lex(src)?);
        let sexps = p.parse_all()?;
        let tops = crate::parse_toplevel(&sexps)?;

        let mut uses = Vec::new();
        let mut items = Vec::new();
        let mut inlines = BTreeMap::new();
        for t in tops {
            match t {
                Top::Use(u) => uses.push(u),
                Top::Inline(inl) => {
                    inlines.insert(inl.name.clone(), inl);
                }
                _ => items.push(t),
            }
        }
        let info = ModuleInfo {
            name: name.to_string(),
            uses,
            inlines,
            tops: items,
        };
        self.modules.insert(name.to_string(), info);
        Ok(())
    }

    fn resolve_all(&mut self) -> DslResult<()> {
        let mut pending: Vec<String> = self.modules.keys().cloned().collect();
        let mut idx = 0usize;
        let builtin_defs = builtin_module_defs();
        while idx < pending.len() {
            let name = pending[idx].clone();
            idx += 1;
            let uses = self.modules.get(&name).unwrap().uses.clone();
            for u in uses {
                let mod_name = module_name_from_import(&u.path)?;
                if !self.modules.contains_key(&mod_name) {
                    if builtin_defs.contains_key(&mod_name) {
                        self.add_builtin_module(&mod_name)?;
                        pending.push(mod_name);
                    } else {
                        let path = self.find_module_path(&u.path)?;
                        let src = std::fs::read_to_string(&path)
                            .map_err(|e| Diag::new(format!("cannot read module {}: {}", u.path, e)))?;
                        self.add_module(&mod_name, &src)?;
                        pending.push(mod_name);
                    }
                }
            }
        }

        let module_names: Vec<String> = self.modules.keys().cloned().collect();
        for name in module_names {
            let info = self.modules.get(&name).unwrap().clone();
            let mut defs = collect_module_defs(&info.tops);
            defs.inlines.extend(info.inlines.keys().cloned());
            self.defs.insert(name.clone(), defs);
            self.inline_defs.insert(name.clone(), info.inlines.clone());
        }

        for (name, defs) in builtin_module_defs() {
            if let Some(existing) = self.defs.get_mut(&name) {
                merge_defs(existing, defs);
            } else {
                self.defs.insert(name, defs);
            }
        }

        let module_names: Vec<String> = self.modules.keys().cloned().collect();
        for name in module_names {
            let info = self.modules.get(&name).unwrap().clone();
            let resolved = resolve_module(&info, &self.defs, &self.inline_defs)?;
            self.modules.get_mut(&name).unwrap().tops = resolved;
        }
        Ok(())
    }

    fn add_builtin_module(&mut self, name: &str) -> DslResult<()> {
        if self.modules.contains_key(name) {
            return Ok(());
        }
        let info = ModuleInfo {
            name: name.to_string(),
            uses: Vec::new(),
            inlines: BTreeMap::new(),
            tops: Vec::new(),
        };
        self.modules.insert(name.to_string(), info);
        Ok(())
    }

    fn find_module_path(&self, import: &str) -> DslResult<PathBuf> {
        let rel = format!("{}.dsl", import.replace('.', "/"));
        for root in &self.lib_paths {
            let candidate = root.join(&rel);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(Diag::new(format!("module not found: {}", import)))
    }

    fn into_tops(self, root: &str) -> DslResult<Vec<Top>> {
        let mut out = Vec::new();
        let mut visited = BTreeSet::new();
        self.emit_module(root, &mut visited, &mut out)?;
        Ok(out)
    }

    fn emit_module(
        &self,
        name: &str,
        visited: &mut BTreeSet<String>,
        out: &mut Vec<Top>,
    ) -> DslResult<()> {
        if visited.contains(name) {
            return Ok(());
        }
        let info = self.modules.get(name).ok_or_else(|| {
            Diag::new(format!("internal error: missing module '{}'", name))
        })?;
        visited.insert(name.to_string());
        for u in &info.uses {
            let mod_name = module_name_from_import(&u.path)?;
            self.emit_module(&mod_name, visited, out)?;
        }
        out.extend(info.tops.clone());
        Ok(())
    }
}

fn collect_module_defs(tops: &[Top]) -> ModuleDefs {
    let mut types = BTreeSet::new();
    let mut variants = BTreeSet::new();
    let mut fns = BTreeSet::new();
    let mut inlines = BTreeSet::new();
    let mut values = BTreeSet::new();
    for t in tops {
        match t {
            Top::Struct(sd) => {
                types.insert(sd.name.clone());
            }
            Top::Union(ud) => {
                types.insert(ud.name.clone());
                for v in &ud.variants {
                    variants.insert(v.name.clone());
                }
            }
            Top::Func(fd) => {
                fns.insert(fd.name.clone());
            }
            Top::Inline(inl) => {
                inlines.insert(inl.name.clone());
            }
            Top::Def(d) => {
                values.insert(d.name.clone());
            }
            Top::Use(_) => {}
        }
    }
    ModuleDefs {
        types,
        variants,
        fns,
        inlines,
        values,
    }
}

fn merge_defs(dst: &mut ModuleDefs, src: ModuleDefs) {
    dst.types.extend(src.types);
    dst.variants.extend(src.variants);
    dst.fns.extend(src.fns);
    dst.inlines.extend(src.inlines);
    dst.values.extend(src.values);
}

fn resolve_module(
    info: &ModuleInfo,
    all: &BTreeMap<String, ModuleDefs>,
    inline_defs: &BTreeMap<String, BTreeMap<String, crate::ast::InlineDef>>,
) -> DslResult<Vec<Top>> {
    let mut out = Vec::new();
    let module = info.name.clone();
    let defs = all.get(&module).cloned().unwrap_or(ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    });

    let resolver = Resolver::new(&module, &defs, &info.uses, all, inline_defs)?;

    for mut t in info.tops.clone() {
        match &mut t {
            Top::Struct(sd) => {
                sd.name = qualify(&module, &sd.name);
                for f in &mut sd.fields {
                    f.ty = resolve_type(&resolver, &f.ty, &f.span)?;
                }
            }
            Top::Union(ud) => {
                ud.name = qualify(&module, &ud.name);
                for v in &mut ud.variants {
                    v.name = qualify(&module, &v.name);
                    for f in &mut v.fields {
                        f.ty = resolve_type(&resolver, &f.ty, &f.span)?;
                    }
                }
            }
            Top::Func(fd) => {
                fd.name = qualify(&module, &fd.name);
                for p in &mut fd.params {
                    if let Some(ann) = &p.ann {
                        p.ann = Some(resolve_type(&resolver, ann, &p.span)?);
                    }
                }
                if let Some(ret) = &fd.extern_ret {
                    fd.extern_ret = Some(resolve_type(&resolver, ret, &fd.span)?);
                }
                fd.body = resolve_expr(&resolver, &fd.body)?;
            }
            Top::Def(d) => {
                d.name = qualify(&module, &d.name);
                if let Some(ann) = &d.ann {
                    d.ann = Some(resolve_type(&resolver, ann, &d.span)?);
                }
                d.expr = resolve_expr(&resolver, &d.expr)?;
            }
            Top::Inline(_) => {}
            Top::Use(_) => {}
        }
        out.push(t);
    }
    Ok(out)
}

fn resolve_expr(res: &Resolver, e: &Expr) -> DslResult<Expr> {
    match e {
        Expr::Call { op, args, span } => {
            if let Some(expanded) = expand_inline_call(res, op, args, span)? {
                return resolve_expr(res, &expanded);
            }
            let mut args_out = Vec::new();
            for a in args {
                args_out.push(resolve_expr(res, a)?);
            }
            let op_res = res.resolve_value_name(op, span)?;
            Ok(Expr::Call {
                op: op_res,
                args: args_out,
                span: span.clone(),
            })
        }
        Expr::Var(name, span) => {
            if let Some(full) = res.resolve_def_name(name, span) {
                Ok(Expr::Var(full, span.clone()))
            } else {
                Ok(e.clone())
            }
        }
        Expr::If {
            cond,
            then_br,
            else_br,
            span,
        } => Ok(Expr::If {
            cond: Box::new(resolve_expr(res, cond)?),
            then_br: Box::new(resolve_expr(res, then_br)?),
            else_br: match else_br {
                Some(b) => Some(Box::new(resolve_expr(res, b)?)),
                None => None,
            },
            span: span.clone(),
        }),
        Expr::Do { exprs, span } => {
            let mut out = Vec::new();
            for e in exprs {
                out.push(resolve_expr(res, e)?);
            }
            Ok(Expr::Do {
                exprs: out,
                span: span.clone(),
            })
        }
        Expr::Loop { body, span } => Ok(Expr::Loop {
            body: Box::new(resolve_expr(res, body)?),
            span: span.clone(),
        }),
        Expr::While { cond, body, span } => Ok(Expr::While {
            cond: Box::new(resolve_expr(res, cond)?),
            body: Box::new(resolve_expr(res, body)?),
            span: span.clone(),
        }),
        Expr::For { var, range, body, span } => Ok(Expr::For {
            var: var.clone(),
            range: resolve_range(res, range)?,
            body: Box::new(resolve_expr(res, body)?),
            span: span.clone(),
        }),
        Expr::Let { bindings, body, span } => {
            let mut out = Vec::new();
            for b in bindings {
                let ann = match &b.ann {
                    Some(t) => Some(resolve_type(res, t, &b.span)?),
                    None => None,
                };
                out.push(crate::ast::LetBinding {
                    name: b.name.clone(),
                    rust_name: b.rust_name.clone(),
                    ann,
                    expr: resolve_expr(res, &b.expr)?,
                    span: b.span.clone(),
                });
            }
            Ok(Expr::Let {
                bindings: out,
                body: Box::new(resolve_expr(res, body)?),
                span: span.clone(),
            })
        }
        Expr::Lambda { params, body, span } => {
            let mut out_params = Vec::new();
            for p in params {
                let ann = match &p.ann {
                    Some(t) => Some(resolve_type(res, t, &p.span)?),
                    None => None,
                };
                out_params.push(crate::ast::Param {
                    name: p.name.clone(),
                    rust_name: p.rust_name.clone(),
                    ann,
                    span: p.span.clone(),
                });
            }
            Ok(Expr::Lambda {
                params: out_params,
                body: Box::new(resolve_expr(res, body)?),
                span: span.clone(),
            })
        }
        Expr::CallDyn { func, args, span } => {
            let func = Box::new(resolve_expr(res, func)?);
            let mut out_args = Vec::new();
            for a in args {
                out_args.push(resolve_expr(res, a)?);
            }
            Ok(Expr::CallDyn {
                func,
                args: out_args,
                span: span.clone(),
            })
        }
        Expr::Break { value, span } => Ok(Expr::Break {
            value: match value {
                Some(v) => Some(Box::new(resolve_expr(res, v)?)),
                None => None,
            },
            span: span.clone(),
        }),
        Expr::Continue { span } => Ok(Expr::Continue { span: span.clone() }),
        Expr::Pair { key, val, span } => Ok(Expr::Pair {
            key: Box::new(resolve_expr(res, key)?),
            val: Box::new(resolve_expr(res, val)?),
            span: span.clone(),
        }),
        Expr::Match { scrutinee, arms, span } => {
            let scrutinee = Box::new(resolve_expr(res, scrutinee)?);
            let mut out_arms = Vec::new();
            for arm in arms {
                let pat = resolve_pat(res, &arm.pat)?;
                let body = resolve_expr(res, &arm.body)?;
                out_arms.push(MatchArm {
                    pat,
                    body,
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
            let mut out_elems = Vec::new();
            for el in elems {
                out_elems.push(resolve_expr(res, el)?);
            }
            let ann = match ann {
                Some(t) => Some(resolve_type(res, t, span)?),
                None => None,
            };
            Ok(Expr::VecLit {
                elems: out_elems,
                span: span.clone(),
                ann,
            })
        }
        Expr::Field { base, field, span } => Ok(Expr::Field {
            base: Box::new(resolve_expr(res, base)?),
            field: field.clone(),
            span: span.clone(),
        }),
        _ => Ok(e.clone()),
    }
}

fn resolve_range(res: &Resolver, range: &crate::ast::RangeExpr) -> DslResult<crate::ast::RangeExpr> {
    Ok(crate::ast::RangeExpr {
        start: Box::new(resolve_expr(res, &range.start)?),
        end: Box::new(resolve_expr(res, &range.end)?),
        step: match &range.step {
            Some(s) => Some(Box::new(resolve_expr(res, s)?)),
            None => None,
        },
        inclusive: range.inclusive,
        span: range.span.clone(),
    })
}

fn expand_inline_call(
    res: &Resolver,
    op: &str,
    args: &[Expr],
    span: &Span,
) -> DslResult<Option<Expr>> {
    let inline = resolve_inline_def(res, op, span)?;
    let inline = match inline {
        Some(v) => v,
        None => return Ok(None),
    };
    if inline.params.len() != args.len() {
        return Err(
            Diag::new(format!(
                "inline '{}' expects {} arguments",
                inline.name,
                inline.params.len()
            ))
            .with_span(span.clone()),
        );
    }
    let mut map = BTreeMap::new();
    for (param, arg) in inline.params.iter().zip(args.iter()) {
        map.insert(param.rust_name.clone(), arg.clone());
    }
    let expanded = inline_subst(&inline.body, &map);
    Ok(Some(expanded))
}

fn resolve_inline_def(res: &Resolver, op: &str, span: &Span) -> DslResult<Option<crate::ast::InlineDef>> {
    if let Some((prefix, item)) = split_qualified(op) {
        let module = res.resolve_module_prefix(prefix, span)?;
        if let Some(mod_inlines) = res.inline_defs.get(&module) {
            return Ok(mod_inlines.get(item).cloned());
        }
        return Ok(None);
    }
    if let Some(mod_inlines) = res.inline_defs.get(&res.module) {
        if let Some(inl) = mod_inlines.get(op) {
            return Ok(Some(inl.clone()));
        }
    }
    for u in &res.uses {
        let mod_name = match module_name_from_import(&u.path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let defs = match res.all.get(&mod_name) {
            Some(d) => d,
            None => continue,
        };
        if u.open || u.only.as_ref().map_or(false, |only| only.contains(&op.to_string())) {
            if defs.inlines.contains(op) {
                if let Some(mod_inlines) = res.inline_defs.get(&mod_name) {
                    if let Some(inl) = mod_inlines.get(op) {
                        return Ok(Some(inl.clone()));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn inline_subst(expr: &Expr, map: &BTreeMap<String, Expr>) -> Expr {
    match expr {
        Expr::Var(name, _) => map.get(name).cloned().unwrap_or_else(|| expr.clone()),
        Expr::Int(..) | Expr::Float(..) | Expr::Str(..) | Expr::Continue { .. } => expr.clone(),
        Expr::Pair { key, val, span } => Expr::Pair {
            key: Box::new(inline_subst(key, map)),
            val: Box::new(inline_subst(val, map)),
            span: span.clone(),
        },
        Expr::Let { bindings, body, span } => {
            let mut out = Vec::new();
            let mut shadowed = map.clone();
            for b in bindings {
                let expr = inline_subst(&b.expr, &shadowed);
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
                body: Box::new(inline_subst(body, &shadowed)),
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
                body: Box::new(inline_subst(body, &shadowed)),
                span: span.clone(),
            }
        }
        Expr::Do { exprs, span } => Expr::Do {
            exprs: exprs.iter().map(|e| inline_subst(e, map)).collect(),
            span: span.clone(),
        },
        Expr::If { cond, then_br, else_br, span } => Expr::If {
            cond: Box::new(inline_subst(cond, map)),
            then_br: Box::new(inline_subst(then_br, map)),
            else_br: else_br.as_ref().map(|b| Box::new(inline_subst(b, map))),
            span: span.clone(),
        },
        Expr::Loop { body, span } => Expr::Loop {
            body: Box::new(inline_subst(body, map)),
            span: span.clone(),
        },
        Expr::While { cond, body, span } => Expr::While {
            cond: Box::new(inline_subst(cond, map)),
            body: Box::new(inline_subst(body, map)),
            span: span.clone(),
        },
        Expr::For { var, range, body, span } => Expr::For {
            var: var.clone(),
            range: inline_subst_range(range, map),
            body: Box::new(inline_subst(body, map)),
            span: span.clone(),
        },
        Expr::Break { value, span } => Expr::Break {
            value: value.as_ref().map(|v| Box::new(inline_subst(v, map))),
            span: span.clone(),
        },
        Expr::Field { base, field, span } => Expr::Field {
            base: Box::new(inline_subst(base, map)),
            field: field.clone(),
            span: span.clone(),
        },
        Expr::Match { scrutinee, arms, span } => {
            let scrutinee = Box::new(inline_subst(scrutinee, map));
            let mut out_arms = Vec::new();
            for arm in arms {
                out_arms.push(MatchArm {
                    pat: arm.pat.clone(),
                    body: inline_subst(&arm.body, map),
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
            args: args.iter().map(|a| inline_subst(a, map)).collect(),
            span: span.clone(),
        },
        Expr::CallDyn { func, args, span } => Expr::CallDyn {
            func: Box::new(inline_subst(func, map)),
            args: args.iter().map(|a| inline_subst(a, map)).collect(),
            span: span.clone(),
        },
        Expr::VecLit { elems, span, ann } => Expr::VecLit {
            elems: elems.iter().map(|e| inline_subst(e, map)).collect(),
            span: span.clone(),
            ann: ann.clone(),
        },
        Expr::MapLit { kind, entries, span, ann } => {
            let entries = entries
                .iter()
                .map(|(k, v)| (inline_subst(k, map), inline_subst(v, map)))
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

fn inline_subst_range(range: &crate::ast::RangeExpr, map: &BTreeMap<String, Expr>) -> crate::ast::RangeExpr {
    crate::ast::RangeExpr {
        start: Box::new(inline_subst(&range.start, map)),
        end: Box::new(inline_subst(&range.end, map)),
        step: range.step.as_ref().map(|s| Box::new(inline_subst(s, map))),
        inclusive: range.inclusive,
        span: range.span.clone(),
    }
}

fn resolve_pat(res: &Resolver, pat: &MatchPat) -> DslResult<MatchPat> {
    match pat {
        MatchPat::Variant { name, bindings, span } => {
            let name = res.resolve_variant_name(name, span)?;
            Ok(MatchPat::Variant {
                name,
                bindings: bindings.clone(),
                span: span.clone(),
            })
        }
        MatchPat::Wildcard(sp) => Ok(MatchPat::Wildcard(sp.clone())),
    }
}

fn resolve_type(res: &Resolver, ty: &Ty, span: &Span) -> DslResult<Ty> {
    match ty {
        Ty::Named(n) => {
            if is_primitive_type(n) {
                return Ok(ty.clone());
            }
            let full = res.resolve_type_name(n, span)?;
            Ok(Ty::Named(full))
        }
        Ty::Vec(inner) => Ok(Ty::Vec(Box::new(resolve_type(res, inner, span)?))),
        Ty::Option(inner) => Ok(Ty::Option(Box::new(resolve_type(res, inner, span)?))),
        Ty::Result(ok, err) => Ok(Ty::Result(
            Box::new(resolve_type(res, ok, span)?),
            Box::new(resolve_type(res, err, span)?),
        )),
        Ty::Map(kind, k, v) => Ok(Ty::Map(
            kind.clone(),
            Box::new(resolve_type(res, k, span)?),
            Box::new(resolve_type(res, v, span)?),
        )),
        Ty::Unknown => Ok(ty.clone()),
    }
}

fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool" | "usize" | "isize" | "()" | "string"
    )
}

fn builtin_module_defs() -> BTreeMap<String, ModuleDefs> {
    let mut out = BTreeMap::new();

    let mut std_io = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    std_io.fns.insert("dbg".to_string());
    out.insert("std.io".to_string(), std_io);

    let mut core_num = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    core_num.fns.insert("abs".to_string());
    core_num.fns.insert("min".to_string());
    core_num.fns.insert("max".to_string());
    core_num.fns.insert("clamp".to_string());
    out.insert("core.num".to_string(), core_num);

    let mut core_vec = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    core_vec.fns.insert("len".to_string());
    core_vec.fns.insert("is-empty".to_string());
    core_vec.fns.insert("get".to_string());
    core_vec.fns.insert("set".to_string());
    out.insert("core.vec".to_string(), core_vec);

    let mut core_str = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    core_str.fns.insert("len".to_string());
    core_str.fns.insert("is-empty".to_string());
    core_str.fns.insert("trim".to_string());
    core_str.fns.insert("split".to_string());
    core_str.fns.insert("join".to_string());
    out.insert("core.str".to_string(), core_str);

    let mut core_fmt = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    core_fmt.fns.insert("dbg".to_string());
    core_fmt.fns.insert("format".to_string());
    core_fmt.fns.insert("pretty".to_string());
    core_fmt.fns.insert("print".to_string());
    core_fmt.fns.insert("println".to_string());
    out.insert("core.fmt".to_string(), core_fmt);

    let mut core_option = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    core_option.fns.insert("some".to_string());
    core_option.fns.insert("none".to_string());
    core_option.fns.insert("is-some".to_string());
    core_option.fns.insert("is-none".to_string());
    core_option.fns.insert("unwrap".to_string());
    core_option.fns.insert("unwrap-or".to_string());
    out.insert("core.option".to_string(), core_option);

    let mut core_result = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    core_result.fns.insert("ok".to_string());
    core_result.fns.insert("err".to_string());
    core_result.fns.insert("is-ok".to_string());
    core_result.fns.insert("is-err".to_string());
    core_result.fns.insert("unwrap".to_string());
    core_result.fns.insert("unwrap-or".to_string());
    out.insert("core.result".to_string(), core_result);

    let mut core_hashmap = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    core_hashmap.fns.insert("new".to_string());
    core_hashmap.fns.insert("len".to_string());
    core_hashmap.fns.insert("is-empty".to_string());
    core_hashmap.fns.insert("get".to_string());
    core_hashmap.fns.insert("contains".to_string());
    core_hashmap.fns.insert("insert".to_string());
    core_hashmap.fns.insert("remove".to_string());
    out.insert("core.hashmap".to_string(), core_hashmap);

    let mut core_btreemap = ModuleDefs {
        types: BTreeSet::new(),
        variants: BTreeSet::new(),
        fns: BTreeSet::new(),
        inlines: BTreeSet::new(),
        values: BTreeSet::new(),
    };
    core_btreemap.fns.insert("new".to_string());
    core_btreemap.fns.insert("len".to_string());
    core_btreemap.fns.insert("is-empty".to_string());
    core_btreemap.fns.insert("get".to_string());
    core_btreemap.fns.insert("contains".to_string());
    core_btreemap.fns.insert("insert".to_string());
    core_btreemap.fns.insert("remove".to_string());
    out.insert("core.btreemap".to_string(), core_btreemap);

    out
}

fn qualify(module: &str, name: &str) -> String {
    format!("{}/{}", module, name)
}

fn module_name_from_import(path: &str) -> DslResult<String> {
    if path.is_empty() {
        return Err(Diag::new("empty module path"));
    }
    if !is_module_path(path) {
        return Err(Diag::new(
            "module path must be lowercase and use lisp-style segments separated by '.'",
        ));
    }
    Ok(path.to_string())
}

fn module_name_from_path(path: &Path) -> DslResult<String> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Diag::new("invalid module path"))?;
    Ok(stem.to_string())
}

#[derive(Debug, Clone)]
struct Resolver {
    module: String,
    uses: Vec<UseDecl>,
    defs: ModuleDefs,
    all: BTreeMap<String, ModuleDefs>,
    inline_defs: BTreeMap<String, BTreeMap<String, crate::ast::InlineDef>>,
}

impl Resolver {
    fn new(
        module: &str,
        defs: &ModuleDefs,
        uses: &[UseDecl],
        all: &BTreeMap<String, ModuleDefs>,
        inline_defs: &BTreeMap<String, BTreeMap<String, crate::ast::InlineDef>>,
    ) -> DslResult<Self> {
        Ok(Self {
            module: module.to_string(),
            uses: uses.to_vec(),
            defs: defs.clone(),
            all: all.clone(),
            inline_defs: inline_defs.clone(),
        })
    }

    fn resolve_value_name(&self, name: &str, span: &Span) -> DslResult<String> {
        if is_builtin_op(name) {
            return Ok(name.to_string());
        }
        if let Some((prefix, item)) = split_qualified(name) {
            let module = self.resolve_module_prefix(prefix, span)?;
            return Ok(qualify(&module, item));
        }
        if self.defs.fns.contains(name) || self.defs.types.contains(name) || self.defs.variants.contains(name) {
            return Ok(qualify(&self.module, name));
        }
        if let Some(full) = self.resolve_callable_from_uses(name) {
            return Ok(full);
        }
        Err(Diag::new(format!("unresolved name '{}'", name)).with_span(span.clone()))
    }

    fn resolve_type_name(&self, name: &str, span: &Span) -> DslResult<String> {
        if let Some((prefix, item)) = split_qualified(name) {
            let module = self.resolve_module_prefix(prefix, span)?;
            return Ok(qualify(&module, item));
        }
        if self.defs.types.contains(name) {
            return Ok(qualify(&self.module, name));
        }
        if let Some(full) = self.resolve_from_uses(name) {
            return Ok(full);
        }
        Err(Diag::new(format!("unresolved type '{}'", name)).with_span(span.clone()))
    }

    fn resolve_variant_name(&self, name: &str, span: &Span) -> DslResult<String> {
        if let Some((prefix, item)) = split_qualified(name) {
            let module = self.resolve_module_prefix(prefix, span)?;
            return Ok(qualify(&module, item));
        }
        if self.defs.variants.contains(name) {
            return Ok(qualify(&self.module, name));
        }
        if let Some(full) = self.resolve_from_uses(name) {
            return Ok(full);
        }
        Err(Diag::new(format!("unresolved variant '{}'", name)).with_span(span.clone()))
    }

    fn resolve_from_uses(&self, name: &str) -> Option<String> {
        for u in &self.uses {
            let mod_name = module_name_from_import(&u.path).ok()?;
            let defs = self.all.get(&mod_name)?;
            if u.open {
                if defs.types.contains(name)
                    || defs.variants.contains(name)
                    || defs.fns.contains(name)
                    || defs.inlines.contains(name)
                {
                    return Some(qualify(&mod_name, name));
                }
            }
            if let Some(only) = &u.only {
                if only.contains(&name.to_string()) {
                    return Some(qualify(&mod_name, name));
                }
            }
        }
        None
    }

    fn resolve_callable_from_uses(&self, name: &str) -> Option<String> {
        for u in &self.uses {
            let mod_name = module_name_from_import(&u.path).ok()?;
            let defs = self.all.get(&mod_name)?;
            if u.open {
                if defs.types.contains(name)
                    || defs.variants.contains(name)
                    || defs.fns.contains(name)
                    || defs.inlines.contains(name)
                {
                    return Some(qualify(&mod_name, name));
                }
            }
            if let Some(only) = &u.only {
                if only.contains(&name.to_string()) {
                    return Some(qualify(&mod_name, name));
                }
            }
        }
        None
    }

    fn resolve_def_name(&self, name: &str, _span: &Span) -> Option<String> {
        if self.defs.values.contains(name) {
            return Some(qualify(&self.module, name));
        }
        self.resolve_def_from_uses(name)
    }

    fn resolve_def_from_uses(&self, name: &str) -> Option<String> {
        for u in &self.uses {
            let mod_name = module_name_from_import(&u.path).ok()?;
            let defs = self.all.get(&mod_name)?;
            if u.open && defs.values.contains(name) {
                return Some(qualify(&mod_name, name));
            }
            if let Some(only) = &u.only {
                if only.contains(&name.to_string()) {
                    if defs.values.contains(name) {
                        return Some(qualify(&mod_name, name));
                    }
                }
            }
        }
        None
    }

    fn resolve_module_prefix(&self, prefix: &str, span: &Span) -> DslResult<String> {
        for u in &self.uses {
            if let Some(alias) = &u.alias {
                if alias == prefix {
                    return module_name_from_import(&u.path);
                }
            }
        }
        let normalized = normalize_module_prefix(prefix);
        if self.all.contains_key(&normalized) {
            return Ok(normalized);
        }
        Err(Diag::new(format!("unknown module '{}'", prefix)).with_span(span.clone()))
    }
}

fn split_qualified(name: &str) -> Option<(&str, &str)> {
    name.rsplit_once('/')
}

fn is_builtin_op(name: &str) -> bool {
    matches!(name, "+" | "-" | "*" | "/" | "dbg")
}

fn normalize_module_prefix(prefix: &str) -> String {
    prefix.replace('/', ".")
}

fn is_module_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    for seg in path.split('.') {
        if seg.is_empty() || !is_lisp_ident(seg) {
            return false;
        }
    }
    true
}

fn is_lisp_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return false;
        }
    }
    true
}

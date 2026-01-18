use crate::diag::{Diag, DslResult, Span};
use crate::lexer::TokKind;
use crate::parser::Sexp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Named(String),
    Vec(Box<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    Map(MapKind, Box<Ty>, Box<Ty>),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapKind {
    Hash,
    BTree,
}

impl Ty {
    pub fn rust(&self) -> String {
        match self {
            Ty::Named(s) => s.clone(),
            Ty::Vec(inner) => format!("Vec<{}>", inner.rust()),
            Ty::Option(inner) => format!("Option<{}>", inner.rust()),
            Ty::Result(ok, err) => format!("Result<{}, {}>", ok.rust(), err.rust()),
            Ty::Map(kind, k, v) => {
                let ty = match kind {
                    MapKind::Hash => "std::collections::HashMap",
                    MapKind::BTree => "std::collections::BTreeMap",
                };
                format!("{}<{}, {}>", ty, k.rust(), v.rust())
            }
            Ty::Unknown => "_".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub rust_name: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub rust_name: String,
    pub fields: Vec<Field>,
    pub span: Span,
    pub extern_: bool,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub rust_name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnionDef {
    pub name: String,
    pub rust_name: String,
    pub variants: Vec<VariantDef>,
    pub span: Span,
    pub extern_: bool,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub rust_name: String,
    pub ann: Option<Ty>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    Str(String, Span),
    Var(String, Span),
    VecLit {
        elems: Vec<Expr>,
        span: Span,
        ann: Option<Ty>,
    },
    Pair {
        key: Box<Expr>,
        val: Box<Expr>,
        span: Span,
    },
    Field {
        base: Box<Expr>,
        field: String,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Call {
        op: String,
        args: Vec<Expr>,
        span: Span,
    },
    MapLit {
        kind: MapKind,
        entries: Vec<(Expr, Expr)>,
        span: Span,
        ann: Option<(Ty, Ty)>,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s) => s.clone(),
            Expr::Float(_, s) => s.clone(),
            Expr::Str(_, s) => s.clone(),
            Expr::Var(_, s) => s.clone(),
            Expr::VecLit { span, .. } => span.clone(),
            Expr::Pair { span, .. } => span.clone(),
            Expr::Field { span, .. } => span.clone(),
            Expr::Match { span, .. } => span.clone(),
            Expr::Call { span, .. } => span.clone(),
            Expr::MapLit { span, .. } => span.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub rust_name: String,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: Span,
    pub extern_: bool,
    pub extern_ret: Option<Ty>,
}

#[derive(Debug, Clone)]
pub enum Top {
    Struct(StructDef),
    Union(UnionDef),
    Func(FnDef),
    Use(UseDecl),
}

#[derive(Debug, Clone)]
pub struct UseDecl {
    pub path: String,
    pub alias: Option<String>,
    pub only: Option<Vec<String>>,
    pub open: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pat: MatchPat,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MatchPat {
    Variant {
        name: String,
        bindings: Vec<(String, String, Span)>,
        span: Span,
    },
    Wildcard(Span),
}

fn atom_sym(se: &Sexp) -> Option<(String, Span)> {
    match se {
        Sexp::Atom(TokKind::Sym(s), sp) => Some((s.clone(), sp.clone())),
        _ => None,
    }
}

fn parse_type_from_sym(s: &str) -> Ty {
    if let Some(inner) = parse_generic_inner(s, "vec") {
        let inner_ty = parse_type_from_sym(inner);
        return Ty::Vec(Box::new(inner_ty));
    }
    if let Some(inner) = parse_generic_inner(s, "option") {
        let inner_ty = parse_type_from_sym(inner);
        return Ty::Option(Box::new(inner_ty));
    }
    if let Some(inner) = parse_generic_inner(s, "result") {
        let args = split_type_args(inner);
        if args.len() == 2 {
            let ok = parse_type_from_sym(args[0]);
            let err = parse_type_from_sym(args[1]);
            return Ty::Result(Box::new(ok), Box::new(err));
        }
    }
    if let Some(inner) = parse_generic_inner(s, "hashmap") {
        let args = split_type_args(inner);
        if args.len() == 2 {
            let k = parse_type_from_sym(args[0]);
            let v = parse_type_from_sym(args[1]);
            return Ty::Map(MapKind::Hash, Box::new(k), Box::new(v));
        }
    }
    if let Some(inner) = parse_generic_inner(s, "btreemap") {
        let args = split_type_args(inner);
        if args.len() == 2 {
            let k = parse_type_from_sym(args[0]);
            let v = parse_type_from_sym(args[1]);
            return Ty::Map(MapKind::BTree, Box::new(k), Box::new(v));
        }
    }
    Ty::Named(s.to_string())
}

fn parse_generic_inner<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    if s.len() <= name.len() + 2 {
        return None;
    }
    let (head, tail) = s.split_at(name.len() + 1);
    if !head[..name.len()].eq_ignore_ascii_case(name) || !head.ends_with('<') || !tail.ends_with('>') {
        return None;
    }
    Some(&tail[..tail.len() - 1])
}

fn split_type_args(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(s[start..].trim());
    out
}

fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "i32"
            | "i64"
            | "u32"
            | "u64"
            | "f32"
            | "f64"
            | "bool"
            | "usize"
            | "isize"
            | "()"
            | "string"
    )
}

fn is_module_path(prefix: &str) -> bool {
    for seg in prefix.split('/') {
        if seg.is_empty() {
            return false;
        }
        for part in seg.split('.') {
            if part.is_empty() || !is_lisp_ident(part) {
                return false;
            }
        }
    }
    true
}

fn ensure_qualified_name(name: &str, span: &Span, kind: &str) -> DslResult<String> {
    if let Some((prefix, item)) = name.rsplit_once('/') {
        if prefix.is_empty() || item.is_empty() || !is_module_path(prefix) || !is_lisp_ident(item) {
            return Err(Diag::new(format!("{} must be a qualified name like mod/name", kind))
                .with_span(span.clone()));
        }
        Ok(name.to_string())
    } else {
        Err(Diag::new(format!("{} must be a qualified name like mod/name", kind))
            .with_span(span.clone()))
    }
}

fn ensure_type_name(name: &str, span: &Span) -> DslResult<String> {
    if name.contains('/') {
        ensure_qualified_name(name, span, "type name")?;
        Ok(name.to_string())
    } else {
        ensure_lisp_ident(name, span, "type name")
    }
}

fn parse_type_from_sym_checked(s: &str, sp: &Span) -> DslResult<Ty> {
    let ty = parse_type_from_sym(s);
    match &ty {
        Ty::Named(n) => {
            if is_primitive_type(n) {
                Ok(ty)
            } else {
                let _ = ensure_type_name(n, sp)?;
                Ok(ty)
            }
        }
        Ty::Vec(inner) => match inner.as_ref() {
            Ty::Named(n) => {
                if is_primitive_type(n) {
                    Ok(ty)
                } else {
                    let _ = ensure_type_name(n, sp)?;
                    Ok(ty)
                }
            }
            _ => Ok(ty),
        },
        Ty::Option(inner) => match inner.as_ref() {
            Ty::Named(n) => {
                if is_primitive_type(n) {
                    Ok(ty)
                } else {
                    let _ = ensure_type_name(n, sp)?;
                    Ok(ty)
                }
            }
            _ => Ok(ty),
        },
        Ty::Result(ok, err) => {
            for t in [ok.as_ref(), err.as_ref()] {
                if let Ty::Named(n) = t {
                    if !is_primitive_type(n) {
                        let _ = ensure_type_name(n, sp)?;
                    }
                }
            }
            Ok(ty)
        }
        Ty::Map(_, k, v) => {
            for t in [k.as_ref(), v.as_ref()] {
                if let Ty::Named(n) = t {
                    if !is_primitive_type(n) {
                        let _ = ensure_type_name(n, sp)?;
                    }
                }
            }
            Ok(ty)
        }
        Ty::Unknown => Ok(ty),
    }
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

fn ensure_lisp_ident(name: &str, span: &Span, kind: &str) -> DslResult<String> {
    if !is_lisp_ident(name) {
        return Err(
            Diag::new(format!("{} must be lowercase lisp-style (kebab-case)", kind))
                .with_span(span.clone()),
        );
    }
    if is_reserved_ident(name) {
        return Err(
            Diag::new(format!("'{}' is a reserved keyword", name)).with_span(span.clone()),
        );
    }
    Ok(name.to_string())
}

fn is_builtin_op(name: &str) -> bool {
    matches!(name, "+" | "-" | "*" | "/" | "dbg")
}

fn is_reserved_ident(name: &str) -> bool {
    matches!(
        name,
        "defn"
            | "defstruct"
            | "defunion"
            | "extern"
            | "match"
            | "use"
            | "open"
            | "vec"
    )
}

fn rust_value_name(name: &str) -> String {
    name.replace('-', "_")
}

fn rust_type_name(name: &str) -> String {
    let mut out = String::new();
    for part in name.split(|c| c == '-' || c == '_') {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(c) = chars.next() {
            out.push(c.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

pub fn parse_struct(se: &Sexp) -> DslResult<StructDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => {
            return Err(Diag::new("expected (defstruct ...)").with_span(se_span(se)))
        }
    };
    if items.len() < 2 {
        return Err(Diag::new("defstruct requires a name").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'defstruct'").with_span(se_span(&items[0])))?;
    if head != "defstruct" {
        return Err(Diag::new("expected 'defstruct'").with_span(se_span(&items[0])));
    }
    let (name, name_sp) = atom_sym(&items[1])
        .ok_or_else(|| Diag::new("expected struct name").with_span(se_span(&items[1])))?;
    let name = ensure_lisp_ident(&name, &name_sp, "struct name")?;

    let mut fields = Vec::new();
    for f in items.iter().skip(2) {
        let (fitems, fspan) = match f {
            Sexp::List(v, sp) => (v, sp.clone()),
            _ => return Err(Diag::new("field must be (name Type)").with_span(se_span(f))),
        };
        if fitems.len() != 2 {
            return Err(Diag::new("field must be (name Type)").with_span(fspan));
        }
        let (fname, fsp) = atom_sym(&fitems[0])
            .ok_or_else(|| Diag::new("expected field name").with_span(se_span(&fitems[0])))?;
        let fname = ensure_lisp_ident(&fname, &fsp, "field name")?;
        let (fty_s, _) = atom_sym(&fitems[1])
            .ok_or_else(|| Diag::new("expected field type").with_span(se_span(&fitems[1])))?;
        fields.push(Field {
            name: fname.clone(),
            rust_name: rust_value_name(&fname),
            ty: parse_type_from_sym_checked(&fty_s, &fspan)?,
            span: Span {
                start: fsp.start,
                end: fspan.end,
            },
        });
    }

    Ok(StructDef {
        name: name.clone(),
        rust_name: rust_type_name(&name),
        fields,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
        extern_: false,
    })
}

pub fn parse_union(se: &Sexp) -> DslResult<UnionDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (defunion ...)").with_span(se_span(se))),
    };
    if items.len() < 2 {
        return Err(Diag::new("defunion requires a name").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'defunion'").with_span(se_span(&items[0])))?;
    if head != "defunion" {
        return Err(Diag::new("expected 'defunion'").with_span(se_span(&items[0])));
    }
    let (name, name_sp) = atom_sym(&items[1])
        .ok_or_else(|| Diag::new("expected union name").with_span(se_span(&items[1])))?;
    let name = ensure_lisp_ident(&name, &name_sp, "union name")?;

    let mut variants = Vec::new();
    for v in items.iter().skip(2) {
        let (vitems, vspan) = match v {
            Sexp::List(v, sp) => (v, sp.clone()),
            _ => {
                return Err(
                    Diag::new("variant must be (Name (field Type) ...)")
                        .with_span(se_span(v)),
                )
            }
        };
        if vitems.is_empty() {
            return Err(Diag::new("variant must have a name").with_span(vspan));
        }
        let (vname, vname_sp) = atom_sym(&vitems[0])
            .ok_or_else(|| Diag::new("expected variant name").with_span(se_span(&vitems[0])))?;
        let vname = ensure_lisp_ident(&vname, &vname_sp, "variant name")?;
        let mut fields = Vec::new();
        for f in vitems.iter().skip(1) {
            let (fitems, fspan) = match f {
                Sexp::List(v, sp) => (v, sp.clone()),
                _ => return Err(Diag::new("field must be (name Type)").with_span(se_span(f))),
            };
            if fitems.len() != 2 {
                return Err(Diag::new("field must be (name Type)").with_span(fspan));
            }
            let (fname, fsp) = atom_sym(&fitems[0])
                .ok_or_else(|| Diag::new("expected field name").with_span(se_span(&fitems[0])))?;
            let fname = ensure_lisp_ident(&fname, &fsp, "field name")?;
        let (fty_s, _) = atom_sym(&fitems[1])
            .ok_or_else(|| Diag::new("expected field type").with_span(se_span(&fitems[1])))?;
        fields.push(Field {
            name: fname.clone(),
            rust_name: rust_value_name(&fname),
            ty: parse_type_from_sym_checked(&fty_s, &fspan)?,
            span: Span {
                start: fsp.start,
                end: fspan.end,
            },
        });
        }
        variants.push(VariantDef {
            name: vname.clone(),
            rust_name: rust_type_name(&vname),
            fields,
            span: Span {
                start: vname_sp.start,
                end: vspan.end,
            },
        });
    }

    Ok(UnionDef {
        name: name.clone(),
        rust_name: rust_type_name(&name),
        variants,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
        extern_: false,
    })
}

pub fn parse_params(se: &Sexp) -> DslResult<Vec<Param>> {
    let (items, _span) = match se {
        Sexp::Brack(items, span) => (items, span.clone()),
        _ => {
            return Err(
                Diag::new("expected parameter list in [..]").with_span(se_span(se))
            )
        }
    };
    let mut out = Vec::new();
    for it in items {
        let (sym, sp) = atom_sym(it).ok_or_else(|| {
            Diag::new("parameter must be a symbol like o:Order").with_span(se_span(it))
        })?;
        let mut parts = sym.splitn(2, ':');
        let name = parts.next().unwrap().to_string();
        let ann = match parts.next() {
            Some(t) => Some(parse_type_from_sym_checked(t, &sp)?),
            None => None,
        };
        if name.is_empty() {
            return Err(Diag::new("invalid parameter name").with_span(sp));
        }
        let name = ensure_lisp_ident(&name, &sp, "parameter name")?;
        let rust_name = rust_value_name(&name);
        out.push(Param {
            name,
            rust_name,
            ann,
            span: sp,
        });
    }
    Ok(out)
}

pub fn parse_expr(se: &Sexp) -> DslResult<Expr> {
    match se {
        Sexp::Atom(TokKind::Int(v), sp) => Ok(Expr::Int(*v, sp.clone())),
        Sexp::Atom(TokKind::Float(v), sp) => Ok(Expr::Float(*v, sp.clone())),
        Sexp::Atom(TokKind::Str(s), sp) => Ok(Expr::Str(s.clone(), sp.clone())),
        Sexp::Atom(TokKind::Sym(s), sp) => {
            if s.contains('/') {
                return Err(Diag::new("qualified name is only allowed in call heads")
                    .with_span(sp.clone()));
            }
            if let Some((a, b)) = s.split_once('.') {
                if !a.is_empty() && !b.is_empty() {
                    let a = ensure_lisp_ident(a, sp, "variable name")?;
                    let b = ensure_lisp_ident(b, sp, "field name")?;
                    return Ok(Expr::Field {
                        base: Box::new(Expr::Var(rust_value_name(&a), sp.clone())),
                        field: rust_value_name(&b),
                        span: sp.clone(),
                    });
                }
            }
            let name = ensure_lisp_ident(s, sp, "variable name")?;
            Ok(Expr::Var(rust_value_name(&name), sp.clone()))
        }
        Sexp::Atom(k, sp) => Err(
            Diag::new(format!("unexpected token {:?} where expression expected", k))
                .with_span(sp.clone()),
        ),
        Sexp::List(items, span) => {
            if items.is_empty() {
                return Err(
                    Diag::new("empty list is not a valid expression").with_span(span.clone())
                );
            }
            let (op, op_span) = match atom_sym(&items[0]) {
                Some(v) => v,
                None => {
                    if items.len() == 2 {
                        let key = parse_expr(&items[0])?;
                        let val = parse_expr(&items[1])?;
                        return Ok(Expr::Pair {
                            key: Box::new(key),
                            val: Box::new(val),
                            span: span.clone(),
                        });
                    }
                    return Err(
                        Diag::new("call head must be a symbol").with_span(se_span(&items[0]))
                    );
                }
            };
            if op.starts_with("core.hashmap/new") || op.starts_with("core.btreemap/new") {
                let (kind, ann) = if let Some(inner) = op.strip_prefix("core.hashmap/new<") {
                    let inner = inner.strip_suffix('>').ok_or_else(|| {
                        Diag::new("map constructor type annotation must end with '>'")
                            .with_span(op_span.clone())
                    })?;
                    let args = split_type_args(inner);
                    if args.len() != 2 {
                        return Err(Diag::new("map type annotation must be hashmap<K,V>")
                            .with_span(op_span.clone()));
                    }
                    let k = parse_type_from_sym_checked(args[0], &op_span)?;
                    let v = parse_type_from_sym_checked(args[1], &op_span)?;
                    (MapKind::Hash, Some((k, v)))
                } else if let Some(inner) = op.strip_prefix("core.btreemap/new<") {
                    let inner = inner.strip_suffix('>').ok_or_else(|| {
                        Diag::new("map constructor type annotation must end with '>'")
                            .with_span(op_span.clone())
                    })?;
                    let args = split_type_args(inner);
                    if args.len() != 2 {
                        return Err(Diag::new("map type annotation must be btreemap<K,V>")
                            .with_span(op_span.clone()));
                    }
                    let k = parse_type_from_sym_checked(args[0], &op_span)?;
                    let v = parse_type_from_sym_checked(args[1], &op_span)?;
                    (MapKind::BTree, Some((k, v)))
                } else if op == "core.hashmap/new" {
                    (MapKind::Hash, None)
                } else {
                    (MapKind::BTree, None)
                };

                let mut entries = Vec::new();
                for item in items.iter().skip(1) {
                    let (pair, pspan) = match item {
                        Sexp::List(v, sp) => (v, sp.clone()),
                        _ => {
                            return Err(
                                Diag::new("map entry must be (key value)").with_span(se_span(item))
                            )
                        }
                    };
                    if pair.len() != 2 {
                        return Err(Diag::new("map entry must be (key value)").with_span(pspan));
                    }
                    let key = parse_expr(&pair[0])?;
                    let val = parse_expr(&pair[1])?;
                    entries.push((key, val));
                }
                return Ok(Expr::MapLit {
                    kind,
                    entries,
                    span: span.clone(),
                    ann,
                });
            }
            if op == "match" {
                return parse_match(span, &items[1..]);
            }
            let head_ty = parse_type_from_sym(&op);
            if let Ty::Vec(_) = head_ty {
                let mut elems = Vec::new();
                for a in items.iter().skip(1) {
                    elems.push(parse_expr(a)?);
                }
                return Ok(Expr::VecLit {
                    elems,
                    span: Span {
                        start: op_span.start,
                        end: span.end.clone(),
                    },
                    ann: Some(head_ty),
                });
            }
            if !is_builtin_op(&op) {
                if op.contains('/') {
                    ensure_qualified_name(&op, &op_span, "call name")?;
                } else {
                    ensure_lisp_ident(&op, &op_span, "call name")?;
                }
            }
            let mut args = Vec::new();
            for a in items.iter().skip(1) {
                args.push(parse_expr(a)?);
            }
            Ok(Expr::Call {
                op,
                args,
                span: span.clone(),
            })
        }
        Sexp::Brack(items, span) => {
            let mut elems = Vec::new();
            for a in items {
                elems.push(parse_expr(a)?);
            }
            Ok(Expr::VecLit {
                elems,
                span: span.clone(),
                ann: None,
            })
        }
    }
}

fn parse_match(span: &Span, items: &[Sexp]) -> DslResult<Expr> {
    if items.len() < 2 {
        return Err(
            Diag::new("match form is (match expr (Variant (...) expr) ...)")
                .with_span(span.clone()),
        );
    }
    let scrutinee = parse_expr(&items[0])?;
    let mut arms = Vec::new();
    for arm in items.iter().skip(1) {
        let (aitems, aspan) = match arm {
            Sexp::List(v, sp) => (v, sp.clone()),
            _ => return Err(Diag::new("match arm must be a list").with_span(se_span(arm))),
        };
        if aitems.len() < 2 {
            return Err(Diag::new("match arm must have a pattern and body").with_span(aspan));
        }
        let (head, head_sp) = atom_sym(&aitems[0])
            .ok_or_else(|| Diag::new("match arm head must be a symbol").with_span(se_span(&aitems[0])))?;
        if head == "_" {
            if aitems.len() != 2 {
                return Err(Diag::new("wildcard match arm is (_ expr)").with_span(aspan));
            }
            let body = parse_expr(&aitems[1])?;
            arms.push(MatchArm {
                pat: MatchPat::Wildcard(head_sp),
                body,
                span: aspan,
            });
            continue;
        }
        let mut bindings = Vec::new();
        let mut idx = 1usize;
        while idx + 1 < aitems.len() {
            if let Sexp::List(bind, _bspan) = &aitems[idx] {
                if bind.len() == 2 {
                    if let Some((field, fsp)) = atom_sym(&bind[0]) {
                        if let Some((name, nsp)) = atom_sym(&bind[1]) {
                            let field = ensure_lisp_ident(&field, &fsp, "field name")?;
                            let field = rust_value_name(&field);
                            let name = if name == "_" {
                                "_".to_string()
                            } else {
                                let n = ensure_lisp_ident(&name, &nsp, "binding name")?;
                                rust_value_name(&n)
                            };
                            bindings.push((field, name, Span { start: fsp.start, end: nsp.end }));
                            idx += 1;
                            continue;
                        }
                    }
                }
                break;
            } else {
                break;
            }
        }
        let head = if head.contains('/') {
            ensure_qualified_name(&head, &head_sp, "variant name")?
        } else {
            ensure_lisp_ident(&head, &head_sp, "variant name")?
        };
        let body = parse_expr(&aitems[idx])?;
        arms.push(MatchArm {
            pat: MatchPat::Variant {
                name: head,
                bindings,
                span: Span {
                    start: head_sp.start,
                    end: aspan.end,
                },
            },
            body,
            span: aspan,
        });
    }
    Ok(Expr::Match {
        scrutinee: Box::new(scrutinee),
        arms,
        span: span.clone(),
    })
}

pub fn parse_fn(se: &Sexp) -> DslResult<FnDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (defn ...)").with_span(se_span(se))),
    };
    if items.len() != 4 {
        return Err(Diag::new("defn form is (defn name [params] body)").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'defn'").with_span(se_span(&items[0])))?;
    if head != "defn" {
        return Err(Diag::new("expected 'defn'").with_span(se_span(&items[0])));
    }
    let (name, name_sp) = atom_sym(&items[1])
        .ok_or_else(|| Diag::new("expected function name").with_span(se_span(&items[1])))?;
    let name = ensure_lisp_ident(&name, &name_sp, "function name")?;
    let params = parse_params(&items[2])?;
    let body = parse_expr(&items[3])?;
    Ok(FnDef {
        name: name.clone(),
        rust_name: rust_value_name(&name),
        params,
        body,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
        extern_: false,
        extern_ret: None,
    })
}

fn atom_str(se: &Sexp) -> Option<(String, Span)> {
    match se {
        Sexp::Atom(TokKind::Str(s), sp) => Some((s.clone(), sp.clone())),
        _ => None,
    }
}

fn parse_extern_toplevel(se: &Sexp) -> DslResult<Top> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (extern ...)").with_span(se_span(se))),
    };
    if items.len() < 2 || items.len() > 3 {
        return Err(Diag::new("extern form is (extern [\"RustName\"] (def...))").with_span(span));
    }
    let mut idx = 1usize;
    let mut override_name = None;
    if let Some((s, _)) = atom_str(&items[1]) {
        override_name = Some(s);
        idx = 2;
    }
    let form = items.get(idx).ok_or_else(|| Diag::new("extern requires a form").with_span(span.clone()))?;
    let (fitems, _fspan) = match form {
        Sexp::List(v, sp) => (v, sp.clone()),
        _ => return Err(Diag::new("extern requires a def form").with_span(se_span(form))),
    };
    if fitems.is_empty() {
        return Err(Diag::new("extern requires a def form").with_span(se_span(form)));
    }
    let (head, _) = atom_sym(&fitems[0])
        .ok_or_else(|| Diag::new("extern form head must be a symbol").with_span(se_span(&fitems[0])))?;
    match head.as_str() {
        "defstruct" => {
            let mut sd = parse_struct(form)?;
            sd.extern_ = true;
            if let Some(rust) = override_name {
                sd.rust_name = rust;
            }
            Ok(Top::Struct(sd))
        }
        "defunion" => {
            let mut ud = parse_union(form)?;
            ud.extern_ = true;
            if let Some(rust) = override_name {
                ud.rust_name = rust;
            }
            Ok(Top::Union(ud))
        }
        "defn" => parse_extern_fn(form, override_name),
        _ => Err(Diag::new("extern can wrap defstruct, defunion, or defn").with_span(se_span(&fitems[0]))),
    }
}

fn parse_extern_fn(se: &Sexp, override_name: Option<String>) -> DslResult<Top> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (defn ...)").with_span(se_span(se))),
    };
    if items.len() != 4 {
        return Err(Diag::new("extern defn form is (defn name [params] RetType)").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'defn'").with_span(se_span(&items[0])))?;
    if head != "defn" {
        return Err(Diag::new("expected 'defn'").with_span(se_span(&items[0])));
    }
    let (name, name_sp) = atom_sym(&items[1])
        .ok_or_else(|| Diag::new("expected function name").with_span(se_span(&items[1])))?;
    let name = ensure_lisp_ident(&name, &name_sp, "function name")?;
    let params = parse_params(&items[2])?;
    let (ret_sym, ret_sp) = atom_sym(&items[3])
        .ok_or_else(|| Diag::new("extern defn return type must be a symbol").with_span(se_span(&items[3])))?;
    let ret_ty = parse_type_from_sym_checked(&ret_sym, &ret_sp)?;
    let dummy_body = Expr::Var("__extern".to_string(), ret_sp.clone());
    Ok(Top::Func(FnDef {
        name: name.clone(),
        rust_name: override_name.unwrap_or_else(|| rust_value_name(&name)),
        params,
        body: dummy_body,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
        extern_: true,
        extern_ret: Some(ret_ty),
    }))
}

fn se_span(se: &Sexp) -> Span {
    match se {
        Sexp::Atom(_, sp) => sp.clone(),
        Sexp::List(_, sp) => sp.clone(),
        Sexp::Brack(_, sp) => sp.clone(),
    }
}

pub fn parse_toplevel(sexps: &[Sexp]) -> DslResult<Vec<Top>> {
    let mut out = Vec::new();
    for se in sexps {
        let (items, span) = match se {
            Sexp::List(items, span) => (items, span.clone()),
            _ => {
                return Err(
                    Diag::new("top-level forms must be lists").with_span(se_span(se))
                )
            }
        };
        if items.is_empty() {
            return Err(Diag::new("empty top-level list").with_span(se_span(se)));
        }
        let (head, _) = atom_sym(&items[0])
            .ok_or_else(|| Diag::new("top-level head must be a symbol").with_span(se_span(&items[0])))?;
        match head.as_str() {
            "use" => out.push(Top::Use(parse_use_decl(&items[1..], &span, false)?)),
            "open" => out.push(Top::Use(parse_use_decl(&items[1..], &span, true)?)),
            "extern" => out.push(parse_extern_toplevel(se)?),
            "defstruct" => out.push(Top::Struct(parse_struct(se)?)),
            "defunion" => out.push(Top::Union(parse_union(se)?)),
            "defn" => out.push(Top::Func(parse_fn(se)?)),
            _ => {
                return Err(Diag::new(format!("unknown top-level form '{}'", head))
                    .with_span(se_span(&items[0])));
            }
        }
    }
    Ok(out)
}

pub fn parse_use_decl(items: &[Sexp], span: &Span, open: bool) -> DslResult<UseDecl> {
    if items.is_empty() {
        return Err(Diag::new("use form is (use path [:as name] [:only (a b)])").with_span(span.clone()));
    }
    let (path, path_sp) = match &items[0] {
        Sexp::Atom(TokKind::Sym(s), sp) => (s.clone(), sp.clone()),
        _ => return Err(Diag::new("use path must be a symbol").with_span(span.clone())),
    };
    let mut alias = None;
    let mut only = None;
    let mut idx = 1usize;
    while idx < items.len() {
        let key = match &items[idx] {
            Sexp::Atom(TokKind::Sym(s), _) => s.clone(),
            _ => return Err(Diag::new("use modifier must be a symbol").with_span(span.clone())),
        };
        idx += 1;
        match key.as_str() {
            ":as" => {
                let (name, sp) = match items.get(idx) {
                    Some(Sexp::Atom(TokKind::Sym(s), sp)) => (s.clone(), sp.clone()),
                    _ => return Err(Diag::new("use :as must be followed by a symbol").with_span(span.clone())),
                };
                let name = ensure_lisp_ident(&name, &sp, "use alias")?;
                alias = Some(name);
                idx += 1;
            }
            ":only" => {
                let list = match items.get(idx) {
                    Some(Sexp::List(v, _)) => v,
                    _ => return Err(Diag::new("use :only must be followed by a list").with_span(span.clone())),
                };
                let mut names = Vec::new();
                for it in list {
                    if let Some((s, sp)) = atom_sym(it) {
                        let s = ensure_lisp_ident(&s, &sp, "use :only name")?;
                        names.push(s);
                    } else {
                        return Err(Diag::new("use :only entries must be symbols").with_span(span.clone()));
                    }
                }
                only = Some(names);
                idx += 1;
            }
            _ => return Err(Diag::new("unknown use modifier").with_span(span.clone())),
        }
    }
    if alias.is_some() && only.is_some() {
        return Err(Diag::new("use cannot combine :as with :only").with_span(span.clone()));
    }
    if open && (alias.is_some() || only.is_some()) {
        return Err(Diag::new("open does not accept :as or :only").with_span(span.clone()));
    }
    Ok(UseDecl { path, alias, only, open, span: path_sp })
}

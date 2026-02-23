use crate::diag::{Diag, DslResult, Loc, Span};
use crate::lexer::TokKind;
use crate::parser::Sexp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Named(String),
    Vec(Box<Ty>),
    Set(Box<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    Map(MapKind, Box<Ty>, Box<Ty>),
    Union(Vec<Ty>),
    Generic(u32),
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
            Ty::Vec(inner) => format!("std::sync::Arc<Vec<{}>>", inner.rust()),
            Ty::Set(inner) => format!("std::collections::HashSet<{}>", inner.rust()),
            Ty::Option(inner) => format!("Option<{}>", inner.rust()),
            Ty::Result(ok, err) => format!("Result<{}, {}>", ok.rust(), err.rust()),
            Ty::Map(kind, k, v) => {
                let ty = match kind {
                    MapKind::Hash => "std::collections::HashMap",
                    MapKind::BTree => "std::collections::BTreeMap",
                };
                format!("{}<{}, {}>", ty, k.rust(), v.rust())
            }
            Ty::Union(_) => "__DarcyUnion".to_string(),
            Ty::Generic(id) => format!("T{}", id),
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
pub enum Iterable {
    Range(RangeExpr),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    Str(String, Span),
    Bool(bool, Span),
    Unit(Span),
    Keyword(String, Span),
    Var(String, Span),
    Ascribe {
        expr: Box<Expr>,
        ann: Ty,
        span: Span,
    },
    Cast {
        expr: Box<Expr>,
        ann: Ty,
        span: Span,
    },
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
    Let {
        bindings: Vec<LetBinding>,
        body: Box<Expr>,
        span: Span,
    },
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
        span: Span,
    },
    CallDyn {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    MethodCall {
        base: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    Do {
        exprs: Vec<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_br: Box<Expr>,
        else_br: Option<Box<Expr>>,
        span: Span,
    },
    Loop {
        body: Box<Expr>,
        span: Span,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    Set {
        name: String,
        expr: Box<Expr>,
        span: Span,
    },
    For {
        var: String,
        iter: Iterable,
        body: Box<Expr>,
        span: Span,
    },
    Break {
        value: Option<Box<Expr>>,
        span: Span,
    },
    Continue {
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
    SetLit {
        elems: Vec<Expr>,
        span: Span,
        ann: Option<Ty>,
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
            Expr::Bool(_, s) => s.clone(),
            Expr::Unit(s) => s.clone(),
            Expr::Keyword(_, s) => s.clone(),
            Expr::Var(_, s) => s.clone(),
            Expr::Ascribe { span, .. } => span.clone(),
            Expr::Cast { span, .. } => span.clone(),
            Expr::VecLit { span, .. } => span.clone(),
            Expr::Pair { span, .. } => span.clone(),
            Expr::Let { span, .. } => span.clone(),
            Expr::Lambda { span, .. } => span.clone(),
            Expr::CallDyn { span, .. } => span.clone(),
            Expr::MethodCall { span, .. } => span.clone(),
            Expr::Do { span, .. } => span.clone(),
            Expr::If { span, .. } => span.clone(),
            Expr::Loop { span, .. } => span.clone(),
            Expr::While { span, .. } => span.clone(),
            Expr::For { span, .. } => span.clone(),
            Expr::Set { span, .. } => span.clone(),
            Expr::Break { span, .. } => span.clone(),
            Expr::Continue { span, .. } => span.clone(),
            Expr::Field { span, .. } => span.clone(),
            Expr::Match { span, .. } => span.clone(),
            Expr::Call { span, .. } => span.clone(),
            Expr::SetLit { span, .. } => span.clone(),
            Expr::MapLit { span, .. } => span.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RangeExpr {
    pub start: Box<Expr>,
    pub end: Box<Expr>,
    pub step: Option<Box<Expr>>,
    pub inclusive: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub rust_name: String,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: Span,
    pub specialize: bool,
    pub exported: bool,
    pub extern_: bool,
    pub extern_ret: Option<Ty>,
}

#[derive(Debug, Clone)]
pub struct InlineDef {
    pub name: String,
    pub rust_name: String,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Def {
    pub name: String,
    pub rust_name: String,
    pub ann: Option<Ty>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub rust_name: String,
    pub ann: Option<Ty>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Top {
    Struct(StructDef),
    Union(UnionDef),
    Func(FnDef),
    Inline(InlineDef),
    Def(Def),
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
    let union_parts = split_union_types(s);
    if union_parts.len() > 1 {
        let tys = union_parts
            .iter()
            .map(|part| parse_type_from_sym(part))
            .collect();
        return Ty::Union(tys);
    }
    if s == "unit" {
        return Ty::Named("()".to_string());
    }
    if let Some(id) = parse_generic_id(s) {
        return Ty::Generic(id);
    }
    if let Some(inner) = parse_generic_inner(s, "vec") {
        let inner_ty = parse_type_from_sym(inner);
        return Ty::Vec(Box::new(inner_ty));
    }
    if let Some(inner) = parse_generic_inner(s, "set").or_else(|| parse_generic_inner(s, "hashset"))
    {
        let inner_ty = parse_type_from_sym(inner);
        return Ty::Set(Box::new(inner_ty));
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
    if let Some(inner) = parse_generic_inner(s, "hash-map") {
        let args = split_type_args(inner);
        if args.len() == 2 {
            let k = parse_type_from_sym(args[0]);
            let v = parse_type_from_sym(args[1]);
            return Ty::Map(MapKind::Hash, Box::new(k), Box::new(v));
        }
    }
    if let Some(inner) = parse_generic_inner(s, "btree-map") {
        let args = split_type_args(inner);
        if args.len() == 2 {
            let k = parse_type_from_sym(args[0]);
            let v = parse_type_from_sym(args[1]);
            return Ty::Map(MapKind::BTree, Box::new(k), Box::new(v));
        }
    }
    Ty::Named(s.to_string())
}

fn split_union_types(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in s.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => {
                if start < idx {
                    out.push(&s[start..idx]);
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

fn parse_generic_id(s: &str) -> Option<u32> {
    let mut chars = s.chars();
    let first = chars.next()?;
    if first != 't' {
        return None;
    }
    let rest: String = chars.collect();
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

fn parse_generic_inner<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    if s.len() <= name.len() + 2 {
        return None;
    }
    let (head, tail) = s.split_at(name.len() + 1);
    if !head[..name.len()].eq_ignore_ascii_case(name)
        || !head.ends_with('<')
        || !tail.ends_with('>')
    {
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
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "f32"
            | "f64"
            | "bool"
            | "usize"
            | "isize"
            | "()"
            | "string"
            | "hashset"
    )
}

fn is_module_path(prefix: &str) -> bool {
    for part in prefix.split('.') {
        if part.is_empty() || !is_lisp_ident(part) {
            return false;
        }
    }
    true
}

fn is_callable_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if first.is_ascii_digit() || first == '/' || first == '.' {
        return false;
    }
    if !is_callable_ident_char(first) {
        return false;
    }
    chars.all(is_callable_ident_char)
}

fn is_callable_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '?' | '!' | '*' | '+' | '=' | '<' | '>' | '$')
}

fn ensure_qualified_name(name: &str, span: &Span, kind: &str) -> DslResult<String> {
    if let Some((prefix, item)) = name.rsplit_once('/') {
        if prefix.is_empty() || item.is_empty() || !is_module_path(prefix) || !is_lisp_ident(item) {
            return Err(
                Diag::new(format!("{} must be a qualified name like mod/name", kind))
                    .with_span(span.clone()),
            );
        }
        Ok(name.to_string())
    } else {
        Err(
            Diag::new(format!("{} must be a qualified name like mod/name", kind))
                .with_span(span.clone()),
        )
    }
}

fn ensure_qualified_callable_name(name: &str, span: &Span, kind: &str) -> DslResult<String> {
    if let Some((prefix, item)) = name.rsplit_once('/') {
        if prefix.is_empty()
            || item.is_empty()
            || !is_module_path(prefix)
            || !is_callable_ident(item)
        {
            return Err(
                Diag::new(format!("{} must be a qualified name like mod/name", kind))
                    .with_span(span.clone()),
            );
        }
        Ok(name.to_string())
    } else {
        Err(
            Diag::new(format!("{} must be a qualified name like mod/name", kind))
                .with_span(span.clone()),
        )
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
        Ty::Set(inner) => match inner.as_ref() {
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
        Ty::Union(items) => {
            for item in items {
                match item {
                    Ty::Named(n) => {
                        if !is_primitive_type(n) {
                            let _ = ensure_type_name(n, sp)?;
                        }
                    }
                    Ty::Vec(inner) | Ty::Set(inner) | Ty::Option(inner) => {
                        if let Ty::Named(n) = inner.as_ref() {
                            if !is_primitive_type(n) {
                                let _ = ensure_type_name(n, sp)?;
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(ty)
        }
        Ty::Generic(_) => Ok(ty),
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
        return Err(Diag::new(format!(
            "{} must be lowercase lisp-style (kebab-case)",
            kind
        ))
        .with_span(span.clone()));
    }
    if is_reserved_ident(name) {
        return Err(Diag::new(format!("'{}' is a reserved keyword", name)).with_span(span.clone()));
    }
    Ok(name.to_string())
}

fn ensure_callable_ident(name: &str, span: &Span, kind: &str) -> DslResult<String> {
    if !is_callable_ident(name) {
        return Err(Diag::new(format!(
            "{} must use valid callable characters (letters, digits, and -_?!*+=<>$)",
            kind
        ))
        .with_span(span.clone()));
    }
    if is_reserved_ident(name) {
        return Err(Diag::new(format!("'{}' is a reserved keyword", name)).with_span(span.clone()));
    }
    Ok(name.to_string())
}

fn ensure_callable_ident_allow_reserved(name: &str, span: &Span, kind: &str) -> DslResult<String> {
    if !is_callable_ident(name) {
        return Err(Diag::new(format!(
            "{} must use valid callable characters (letters, digits, and -_?!*+=<>$)",
            kind
        ))
        .with_span(span.clone()));
    }
    Ok(name.to_string())
}

fn ensure_keyword_ident(name: &str, span: &Span) -> DslResult<String> {
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '?' || c == '!')
    {
        return Ok(name.to_string());
    }
    Err(Diag::new("invalid keyword identifier").with_span(span.clone()))
}

fn ensure_member_ident(name: &str, span: &Span) -> DslResult<String> {
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        if name
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            return Err(
                Diag::new("member identifier cannot start with a digit").with_span(span.clone())
            );
        }
        return Ok(name.to_string());
    }
    if !is_lisp_ident(name) {
        return Err(
            Diag::new("keyword must be lowercase lisp-style (kebab-case)").with_span(span.clone()),
        );
    }
    Ok(name.to_string())
}

fn is_builtin_op(name: &str) -> bool {
    matches!(
        name,
        "+" | "-" | "*" | "/" | "mod" | "=" | "<" | ">" | "<=" | ">=" | "&" | "|"
    )
}

fn is_reserved_ident(name: &str) -> bool {
    matches!(
        name,
        "def"
            | "defn"
            | "defpub"
            | "defin"
            | "defstruct"
            | "defrecord"
            | "defunion"
            | "defenum"
            | "extern"
            | "match"
            | "export"
            | "quote"
            | "syntax-quote"
            | "unquote"
            | "unquote-splicing"
            | "with-meta"
            | "type"
            | "cast"
            | "case"
            | "cond"
            | "and"
            | "or"
            | "when"
            | "if"
            | "do"
            | "loop"
            | "while"
            | "for"
            | "break"
            | "continue"
            | "let"
            | "let!"
            | "fn"
            | "call"
            | "require"
            | "true"
            | "false"
            | "nil"
            | "list"
            | "vec"
            | "set"
            | "hashset"
            | "range"
            | "range-incl"
    )
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
        _ => return Err(Diag::new("expected (defrecord ...)").with_span(se_span(se))),
    };
    if items.len() < 2 {
        return Err(Diag::new("defrecord requires a name").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'defrecord'").with_span(se_span(&items[0])))?;
    if head != "defrecord" {
        return Err(Diag::new("expected 'defrecord'").with_span(se_span(&items[0])));
    }
    let (name, name_sp) = atom_sym(&items[1])
        .ok_or_else(|| Diag::new("expected struct name").with_span(se_span(&items[1])))?;
    let name = ensure_lisp_ident(&name, &name_sp, "struct name")?;

    let fields = parse_decl_fields(items.iter().skip(2), "field")?;

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
        _ => return Err(Diag::new("expected (defenum ...)").with_span(se_span(se))),
    };
    if items.len() < 2 {
        return Err(Diag::new("defenum requires a name").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'defenum'").with_span(se_span(&items[0])))?;
    if head != "defenum" {
        return Err(Diag::new("expected 'defenum'").with_span(se_span(&items[0])));
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
                    Diag::new("variant must be (Name (field Type) ...)").with_span(se_span(v))
                )
            }
        };
        if vitems.is_empty() {
            return Err(Diag::new("variant must have a name").with_span(vspan));
        }
        let (vname, vname_sp) = atom_sym(&vitems[0])
            .ok_or_else(|| Diag::new("expected variant name").with_span(se_span(&vitems[0])))?;
        let vname = ensure_lisp_ident(&vname, &vname_sp, "variant name")?;
        let fields = parse_decl_fields(vitems.iter().skip(1), "field")?;
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

fn parse_decl_fields<'a, I>(forms: I, kind: &str) -> DslResult<Vec<Field>>
where
    I: Iterator<Item = &'a Sexp>,
{
    let mut out = Vec::new();
    for form in forms {
        match form {
            // Legacy style: (name Type) or (name)
            Sexp::List(items, fspan) => {
                if items.is_empty() || items.len() > 2 {
                    return Err(Diag::new(format!(
                        "{} must be (name Type), (name), [name:type], or [name:type ...]",
                        kind
                    ))
                    .with_span(fspan.clone()));
                }
                let (fname, fsp) = atom_sym(&items[0]).ok_or_else(|| {
                    Diag::new(format!("expected {} name", kind)).with_span(se_span(&items[0]))
                })?;
                let fname = ensure_lisp_ident(&fname, &fsp, &format!("{} name", kind))?;
                let fty = if items.len() == 2 {
                    let (fty_s, _) = atom_sym(&items[1]).ok_or_else(|| {
                        Diag::new(format!("expected {} type", kind)).with_span(se_span(&items[1]))
                    })?;
                    parse_type_from_sym_checked(&fty_s, fspan)?
                } else {
                    Ty::Unknown
                };
                out.push(Field {
                    name: fname.clone(),
                    rust_name: rust_value_name(&fname),
                    ty: fty,
                    span: Span {
                        start: fsp.start,
                        end: fspan.end,
                    },
                });
            }
            // New style:
            //   [x:type]
            //   [x:type y:type]
            //   [x type y type]
            //   [x type]
            Sexp::Brack(items, bspan) => {
                if items.is_empty() {
                    return Err(Diag::new(format!(
                        "{} declaration cannot be empty",
                        kind
                    ))
                    .with_span(bspan.clone()));
                }
                let mut syms: Vec<(String, Span)> = Vec::new();
                for it in items {
                    let (sym, sp) = atom_sym(it).ok_or_else(|| {
                        Diag::new(format!("{} declaration must use symbols", kind))
                            .with_span(se_span(it))
                    })?;
                    syms.push((sym, sp));
                }
                if syms.iter().all(|(s, _)| s.contains(':')) {
                    for (sym, sp) in syms {
                        let (name_raw, ty_raw) = sym.split_once(':').ok_or_else(|| {
                            Diag::new(format!("invalid {} declaration", kind)).with_span(sp.clone())
                        })?;
                        if name_raw.is_empty() || ty_raw.is_empty() {
                            return Err(Diag::new(format!(
                                "{} declaration must be name:Type",
                                kind
                            ))
                            .with_span(sp.clone()));
                        }
                        let name = ensure_lisp_ident(name_raw, &sp, &format!("{} name", kind))?;
                        let ty = parse_type_from_sym_checked(ty_raw, &sp)?;
                        out.push(Field {
                            name: name.clone(),
                            rust_name: rust_value_name(&name),
                            ty,
                            span: sp,
                        });
                    }
                    continue;
                }
                if syms.len() == 1 {
                    let (name_raw, sp) = &syms[0];
                    let name = ensure_lisp_ident(name_raw, sp, &format!("{} name", kind))?;
                    out.push(Field {
                        name: name.clone(),
                        rust_name: rust_value_name(&name),
                        ty: Ty::Unknown,
                        span: sp.clone(),
                    });
                    continue;
                }
                if syms.len() % 2 != 0 {
                    return Err(Diag::new(format!(
                        "{} declarations must be name/type pairs or name:Type",
                        kind
                    ))
                    .with_span(bspan.clone()));
                }
                let mut idx = 0usize;
                while idx < syms.len() {
                    let (name_raw, nsp) = &syms[idx];
                    let (ty_raw, tsp) = &syms[idx + 1];
                    let name = ensure_lisp_ident(name_raw, nsp, &format!("{} name", kind))?;
                    let ty = parse_type_from_sym_checked(ty_raw, tsp)?;
                    out.push(Field {
                        name: name.clone(),
                        rust_name: rust_value_name(&name),
                        ty,
                        span: Span {
                            start: nsp.start,
                            end: tsp.end,
                        },
                    });
                    idx += 2;
                }
            }
            _ => {
                return Err(Diag::new(format!(
                    "{} must be (name Type), (name), [name:type], or [name:type ...]",
                    kind
                ))
                .with_span(se_span(form)))
            }
        }
    }
    Ok(out)
}

pub fn parse_params(se: &Sexp) -> DslResult<Vec<Param>> {
    let (items, _span) = match se {
        Sexp::Brack(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected parameter list in [..]").with_span(se_span(se))),
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

fn parse_symbol_ascription(sym: &str, sp: &Span) -> DslResult<Option<Expr>> {
    let (name, ty) = match sym.split_once(':') {
        Some(parts) => parts,
        None => return Ok(None),
    };
    if name.is_empty() || ty.is_empty() {
        return Err(Diag::new("type ascription must be name:Type").with_span(sp.clone()));
    }
    let name = ensure_lisp_ident(name, sp, "variable name")?;
    let ann = parse_type_from_sym_checked(ty, sp)?;
    Ok(Some(Expr::Ascribe {
        expr: Box::new(Expr::Var(rust_value_name(&name), sp.clone())),
        ann,
        span: sp.clone(),
    }))
}

pub fn parse_expr(se: &Sexp) -> DslResult<Expr> {
    match se {
        Sexp::Atom(TokKind::Int(v), sp) => Ok(Expr::Int(*v, sp.clone())),
        Sexp::Atom(TokKind::Float(v), sp) => Ok(Expr::Float(*v, sp.clone())),
        Sexp::Atom(TokKind::Str(s), sp) => Ok(Expr::Str(s.clone(), sp.clone())),
        Sexp::Atom(TokKind::Sym(s), sp) if s == "true" => Ok(Expr::Bool(true, sp.clone())),
        Sexp::Atom(TokKind::Sym(s), sp) if s == "false" => Ok(Expr::Bool(false, sp.clone())),
        Sexp::Atom(TokKind::Sym(s), sp) if s == "nil" => Ok(Expr::Unit(sp.clone())),
        Sexp::Atom(TokKind::Sym(s), sp) if s.starts_with(':') && s.len() > 1 => {
            Ok(Expr::Keyword(s.clone(), sp.clone()))
        }
        Sexp::Atom(TokKind::Sym(s), sp) => {
            if s == "true" {
                return Ok(Expr::Bool(true, sp.clone()));
            }
            if s == "false" {
                return Ok(Expr::Bool(false, sp.clone()));
            }
            if s == "nil" {
                return Ok(Expr::Unit(sp.clone()));
            }
            if let Some(rest) = s.strip_prefix(':') {
                if rest.is_empty() {
                    return Err(Diag::new("keyword cannot be empty").with_span(sp.clone()));
                }
                let name = ensure_keyword_ident(rest, sp)?;
                return Ok(Expr::Keyword(format!(":{}", name), sp.clone()));
            }
            if let Some(expr) = parse_symbol_ascription(s, sp)? {
                return Ok(expr);
            }
            if s.contains('/') {
                return Err(
                    Diag::new("qualified name is only allowed in call heads").with_span(sp.clone())
                );
            }
            if let Some((a, b)) = s.split_once('.') {
                if !a.is_empty() && !b.is_empty() {
                    let a = ensure_lisp_ident(a, sp, "variable name")?;
                    let b = ensure_lisp_ident(b, sp, "field name")?;
                    let base_end = Loc {
                        line: sp.start.line,
                        col: sp.start.col + a.len(),
                        byte: sp.start.byte + a.len(),
                    };
                    let base_span = Span {
                        start: sp.start,
                        end: base_end,
                    };
                    return Ok(Expr::Field {
                        base: Box::new(Expr::Var(rust_value_name(&a), base_span)),
                        field: rust_value_name(&b),
                        span: sp.clone(),
                    });
                }
            }
            let name = ensure_lisp_ident(s, sp, "variable name")?;
            Ok(Expr::Var(rust_value_name(&name), sp.clone()))
        }
        Sexp::Atom(k, sp) => Err(Diag::new(format!(
            "unexpected token {:?} where expression expected",
            k
        ))
        .with_span(sp.clone())),
        Sexp::List(items, span) => {
            if items.is_empty() {
                return Err(
                    Diag::new("empty list is not a valid expression").with_span(span.clone())
                );
            }
            if items.len() == 1 {
                if let Some((sym, sym_span)) = atom_sym(&items[0]) {
                    if let Some(expr) = parse_symbol_ascription(&sym, &sym_span)? {
                        return Ok(expr);
                    }
                }
            }
            let (op, op_span) = match atom_sym(&items[0]) {
                Some(v) => v,
                None => {
                    return Err(
                        Diag::new("call head must be a symbol").with_span(se_span(&items[0]))
                    )
                }
            };
            if op == "." {
                if items.len() < 3 {
                    return Err(
                        Diag::new("interop form is (. obj member [args...])").with_span(op_span)
                    );
                }
                let base = parse_expr(&items[1])?;
                let (member, msp) = atom_sym(&items[2]).ok_or_else(|| {
                    Diag::new("member name must be a symbol").with_span(se_span(&items[2]))
                })?;
                let member = ensure_member_ident(&member, &msp)?;
                let mut args = Vec::new();
                for item in items.iter().skip(3) {
                    args.push(parse_expr(item)?);
                }
                return Ok(Expr::MethodCall {
                    base: Box::new(base),
                    method: rust_value_name(&member),
                    args,
                    span: span.clone(),
                });
            }
            if let Some(method) = op.strip_prefix('.') {
                if method.is_empty() {
                    return Err(
                        Diag::new("interop form is (.method obj [args...])").with_span(op_span)
                    );
                }
                if items.len() < 2 {
                    return Err(
                        Diag::new("interop form is (.method obj [args...])").with_span(op_span)
                    );
                }
                let method = ensure_member_ident(method, &op_span)?;
                let base = parse_expr(&items[1])?;
                let mut args = Vec::new();
                for item in items.iter().skip(2) {
                    args.push(parse_expr(item)?);
                }
                return Ok(Expr::MethodCall {
                    base: Box::new(base),
                    method: rust_value_name(&method),
                    args,
                    span: span.clone(),
                });
            }
            if op == "if" {
                if items.len() < 3 || items.len() > 4 {
                    return Err(Diag::new("if form is (if cond then [else])").with_span(op_span));
                }
                let cond = parse_expr(&items[1])?;
                let then_br = parse_expr(&items[2])?;
                let else_br = if items.len() == 4 {
                    Some(Box::new(parse_expr(&items[3])?))
                } else {
                    None
                };
                return Ok(Expr::If {
                    cond: Box::new(cond),
                    then_br: Box::new(then_br),
                    else_br,
                    span: span.clone(),
                });
            }
            if op == "type" {
                if items.len() != 3 {
                    return Err(Diag::new("type form is (type expr Type)").with_span(op_span));
                }
                let expr = parse_expr(&items[1])?;
                let (ty_sym, ty_sp) = atom_sym(&items[2]).ok_or_else(|| {
                    Diag::new("type name must be a symbol").with_span(se_span(&items[2]))
                })?;
                let ann = parse_type_from_sym_checked(&ty_sym, &ty_sp)?;
                return Ok(Expr::Ascribe {
                    expr: Box::new(expr),
                    ann,
                    span: span.clone(),
                });
            }
            if op == "cast" {
                if items.len() != 3 {
                    return Err(Diag::new("cast form is (cast expr Type)").with_span(op_span));
                }
                let expr = parse_expr(&items[1])?;
                let (ty_sym, ty_sp) = atom_sym(&items[2]).ok_or_else(|| {
                    Diag::new("cast type must be a symbol").with_span(se_span(&items[2]))
                })?;
                let ann = parse_type_from_sym_checked(&ty_sym, &ty_sp)?;
                return Ok(Expr::Cast {
                    expr: Box::new(expr),
                    ann,
                    span: span.clone(),
                });
            }
            if op == "and" {
                return parse_and(span, &items[1..]);
            }
            if op == "or" {
                return parse_or(span, &items[1..]);
            }
            if op == "when" {
                if items.len() < 3 {
                    return Err(Diag::new("when form is (when cond expr ...)").with_span(op_span));
                }
                let cond = parse_expr(&items[1])?;
                let body = if items.len() == 3 {
                    parse_expr(&items[2])?
                } else {
                    let mut exprs = Vec::new();
                    for it in items.iter().skip(2) {
                        exprs.push(parse_expr(it)?);
                    }
                    Expr::Do {
                        exprs,
                        span: span.clone(),
                    }
                };
                return Ok(Expr::If {
                    cond: Box::new(cond),
                    then_br: Box::new(body),
                    else_br: None,
                    span: span.clone(),
                });
            }
            if op == "type" {
                if items.len() != 3 {
                    return Err(Diag::new("type form is (type expr Type)").with_span(op_span));
                }
                let expr = parse_expr(&items[1])?;
                let (ty_sym, ty_sp) = atom_sym(&items[2]).ok_or_else(|| {
                    Diag::new("type name must be a symbol").with_span(se_span(&items[2]))
                })?;
                let ann = parse_type_from_sym_checked(&ty_sym, &ty_sp)?;
                return Ok(Expr::Ascribe {
                    expr: Box::new(expr),
                    ann,
                    span: span.clone(),
                });
            }
            if op == "cast" {
                if items.len() != 3 {
                    return Err(Diag::new("cast form is (cast expr Type)").with_span(op_span));
                }
                let expr = parse_expr(&items[1])?;
                let (ty_sym, ty_sp) = atom_sym(&items[2]).ok_or_else(|| {
                    Diag::new("cast type must be a symbol").with_span(se_span(&items[2]))
                })?;
                let ann = parse_type_from_sym_checked(&ty_sym, &ty_sp)?;
                return Ok(Expr::Cast {
                    expr: Box::new(expr),
                    ann,
                    span: span.clone(),
                });
            }
            if op == "with-meta" {
                if items.len() != 3 {
                    return Err(
                        Diag::new("with-meta form is (with-meta form meta)").with_span(op_span)
                    );
                }
                return parse_expr(&items[1]);
            }
            if op == "loop" {
                if items.len() < 2 {
                    return Err(Diag::new("loop form is (loop expr ...)").with_span(op_span));
                }
                let body = parse_body_expr(&items[1..], span)?;
                return Ok(Expr::Loop {
                    body: Box::new(body),
                    span: span.clone(),
                });
            }
            if op == "while" {
                if items.len() < 3 {
                    return Err(Diag::new("while form is (while cond expr ...)").with_span(op_span));
                }
                let cond = parse_expr(&items[1])?;
                let body = parse_body_expr(&items[2..], span)?;
                return Ok(Expr::While {
                    cond: Box::new(cond),
                    body: Box::new(body),
                    span: span.clone(),
                });
            }
            if op == "for" {
                if items.len() < 4 {
                    return Err(
                        Diag::new("for form is (for name iterable expr ...)").with_span(op_span)
                    );
                }
                let (name, sp) = atom_sym(&items[1]).ok_or_else(|| {
                    Diag::new("for binding must be a symbol").with_span(se_span(&items[1]))
                })?;
                let name = ensure_lisp_ident(&name, &sp, "for binding")?;

                let iter = if is_range_form(&items[2]) {
                    Iterable::Range(parse_range_expr(&items[2])?)
                } else {
                    Iterable::Expr(Box::new(parse_expr(&items[2])?))
                };

                let body = parse_body_expr(&items[3..], span)?;
                return Ok(Expr::For {
                    var: rust_value_name(&name),
                    iter,
                    body: Box::new(body),
                    span: span.clone(),
                });
            }
            if op == "let!" {
                if items.len() != 3 {
                    return Err(Diag::new("let! form is (let! name expr)").with_span(op_span));
                }
                let (name, sp) = atom_sym(&items[1]).ok_or_else(|| {
                    Diag::new("let! binding must be a symbol").with_span(se_span(&items[1]))
                })?;
                let name = ensure_lisp_ident(&name, &sp, "let! binding")?;
                let expr = parse_expr(&items[2])?;
                return Ok(Expr::Set {
                    name: rust_value_name(&name),
                    expr: Box::new(expr),
                    span: span.clone(),
                });
            }
            if op == "break" {
                if items.len() > 2 {
                    return Err(Diag::new("break form is (break [expr])").with_span(op_span));
                }
                let value = if items.len() == 2 {
                    Some(Box::new(parse_expr(&items[1])?))
                } else {
                    None
                };
                return Ok(Expr::Break {
                    value,
                    span: span.clone(),
                });
            }
            if op == "continue" {
                if items.len() != 1 {
                    return Err(Diag::new("continue form is (continue)").with_span(op_span));
                }
                return Ok(Expr::Continue { span: span.clone() });
            }
            if op.starts_with("darcy.hash-map/new") || op.starts_with("darcy.btree-map/new") {
                let (kind, ann) = if let Some(inner) = op.strip_prefix("darcy.hash-map/new<") {
                    let inner = inner.strip_suffix('>').ok_or_else(|| {
                        Diag::new("map constructor type annotation must end with '>'")
                            .with_span(op_span.clone())
                    })?;
                    let args = split_type_args(inner);
                    if args.len() != 2 {
                        return Err(Diag::new("map type annotation must be hash-map<K,V>")
                            .with_span(op_span.clone()));
                    }
                    let k = parse_type_from_sym_checked(args[0], &op_span)?;
                    let v = parse_type_from_sym_checked(args[1], &op_span)?;
                    (MapKind::Hash, Some((k, v)))
                } else if let Some(inner) = op.strip_prefix("darcy.btree-map/new<") {
                    let inner = inner.strip_suffix('>').ok_or_else(|| {
                        Diag::new("map constructor type annotation must end with '>'")
                            .with_span(op_span.clone())
                    })?;
                    let args = split_type_args(inner);
                    if args.len() != 2 {
                        return Err(Diag::new("map type annotation must be btree-map<K,V>")
                            .with_span(op_span.clone()));
                    }
                    let k = parse_type_from_sym_checked(args[0], &op_span)?;
                    let v = parse_type_from_sym_checked(args[1], &op_span)?;
                    (MapKind::BTree, Some((k, v)))
                } else if op == "darcy.hash-map/new" {
                    (MapKind::Hash, None)
                } else {
                    (MapKind::BTree, None)
                };

                let mut entries = Vec::new();
                for item in items.iter().skip(1) {
                    let (pair, pspan) = match item {
                        Sexp::Brack(v, sp) => (v, sp.clone()),
                        _ => {
                            return Err(
                                Diag::new("map entry must be [key value]").with_span(se_span(item))
                            )
                        }
                    };
                    if pair.len() != 2 {
                        return Err(Diag::new("map entry must be [key value]").with_span(pspan));
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
            if op == "case" {
                return parse_match(span, &items[1..]);
            }
            if op == "cond" {
                return parse_cond(span, &items[1..]);
            }
            if op == "let" {
                if items.len() < 3 {
                    return Err(
                        Diag::new("let form is (let [bindings] expr ...)").with_span(op_span)
                    );
                }
                let mut all_bindings = Vec::new();
                let mut body_start = 1usize;
                while body_start < items.len() {
                    if matches!(items[body_start], Sexp::Brack(_, _)) {
                        let mut part = parse_let_bindings(&items[body_start])?;
                        all_bindings.append(&mut part);
                        body_start += 1;
                    } else {
                        break;
                    }
                }
                if all_bindings.is_empty() || body_start >= items.len() {
                    return Err(
                        Diag::new("let form is (let [bindings] expr ...)").with_span(op_span)
                    );
                }
                let body = parse_body_expr(&items[body_start..], span)?;
                return Ok(Expr::Let {
                    bindings: all_bindings,
                    body: Box::new(body),
                    span: span.clone(),
                });
            }
            if op == "fn" {
                if items.len() < 3 {
                    return Err(Diag::new("fn form is (fn [params] expr ...)").with_span(op_span));
                }
                let params = parse_params(&items[1])?;
                let body = parse_body_expr(&items[2..], span)?;
                return Ok(Expr::Lambda {
                    params,
                    body: Box::new(body),
                    span: span.clone(),
                });
            }
            if op == "call" {
                if items.len() < 2 {
                    return Err(Diag::new("call form is (call f arg ...)").with_span(op_span));
                }
                let func = parse_expr(&items[1])?;
                let mut args = Vec::new();
                for item in items.iter().skip(2) {
                    args.push(parse_expr(item)?);
                }
                return Ok(Expr::CallDyn {
                    func: Box::new(func),
                    args,
                    span: span.clone(),
                });
            }
            if op == "do" {
                if items.len() < 2 {
                    return Err(Diag::new("do form is (do expr ...)").with_span(op_span));
                }
                let mut exprs = Vec::new();
                for item in items.iter().skip(1) {
                    exprs.push(parse_expr(item)?);
                }
                return Ok(Expr::Do {
                    exprs,
                    span: span.clone(),
                });
            }
            if op == "list" {
                let mut elems = Vec::new();
                for a in items.iter().skip(1) {
                    elems.push(parse_expr(a)?);
                }
                return Ok(Expr::VecLit {
                    elems,
                    span: span.clone(),
                    ann: None,
                });
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
            if let Ty::Set(_) = head_ty {
                let mut elems = Vec::new();
                for a in items.iter().skip(1) {
                    elems.push(parse_expr(a)?);
                }
                return Ok(Expr::SetLit {
                    elems,
                    span: Span {
                        start: op_span.start,
                        end: span.end.clone(),
                    },
                    ann: Some(head_ty),
                });
            }
            if op == "set" || op == "hashset" {
                let mut elems = Vec::new();
                for a in items.iter().skip(1) {
                    elems.push(parse_expr(a)?);
                }
                return Ok(Expr::SetLit {
                    elems,
                    span: span.clone(),
                    ann: None,
                });
            }
            if !is_builtin_op(&op) {
                if op.contains('/') {
                    ensure_qualified_callable_name(&op, &op_span, "call name")?;
                } else {
                    ensure_callable_ident(&op, &op_span, "call name")?;
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
        Sexp::Brace(items, span) => {
            if items.len() % 2 != 0 {
                return Err(
                    Diag::new("map literal requires even number of forms").with_span(span.clone())
                );
            }
            let mut entries = Vec::new();
            let mut iter = items.iter();
            while let Some(key) = iter.next() {
                let val = iter.next().expect("even number of map entries");
                entries.push((parse_expr(key)?, parse_expr(val)?));
            }
            Ok(Expr::MapLit {
                kind: MapKind::Hash,
                entries,
                span: span.clone(),
                ann: None,
            })
        }
        Sexp::Set(items, span) => {
            let mut elems = Vec::new();
            for item in items {
                elems.push(parse_expr(item)?);
            }
            Ok(Expr::SetLit {
                elems,
                span: span.clone(),
                ann: None,
            })
        }
    }
}

fn parse_and(span: &Span, items: &[Sexp]) -> DslResult<Expr> {
    parse_and_inner(span, items, 0)
}

fn parse_and_inner(span: &Span, items: &[Sexp], depth: usize) -> DslResult<Expr> {
    if items.is_empty() {
        return Ok(Expr::Bool(true, span.clone()));
    }
    let first = parse_expr(&items[0])?;
    if items.len() == 1 {
        return Ok(first);
    }
    let name = format!("and-temp-{}-{}", span.start.line, depth);
    let rust_name = rust_value_name(&name);
    let binding = LetBinding {
        name: name.clone(),
        rust_name: rust_name.clone(),
        ann: None,
        expr: first,
        span: span.clone(),
    };
    let rest = parse_and_inner(span, &items[1..], depth + 1)?;
    let var = Expr::Var(rust_name.clone(), span.clone());
    let if_expr = Expr::If {
        cond: Box::new(var.clone()),
        then_br: Box::new(rest),
        else_br: Some(Box::new(var)),
        span: span.clone(),
    };
    Ok(Expr::Let {
        bindings: vec![binding],
        body: Box::new(if_expr),
        span: span.clone(),
    })
}

fn parse_or(span: &Span, items: &[Sexp]) -> DslResult<Expr> {
    parse_or_inner(span, items, 0)
}

fn parse_or_inner(span: &Span, items: &[Sexp], depth: usize) -> DslResult<Expr> {
    if items.is_empty() {
        return Ok(Expr::Unit(span.clone()));
    }
    let first = parse_expr(&items[0])?;
    if items.len() == 1 {
        return Ok(first);
    }
    let name = format!("or-temp-{}-{}", span.start.line, depth);
    let rust_name = rust_value_name(&name);
    let binding = LetBinding {
        name: name.clone(),
        rust_name: rust_name.clone(),
        ann: None,
        expr: first,
        span: span.clone(),
    };
    let rest = parse_or_inner(span, &items[1..], depth + 1)?;
    let var = Expr::Var(rust_name.clone(), span.clone());
    let if_expr = Expr::If {
        cond: Box::new(var.clone()),
        then_br: Box::new(var),
        else_br: Some(Box::new(rest)),
        span: span.clone(),
    };
    Ok(Expr::Let {
        bindings: vec![binding],
        body: Box::new(if_expr),
        span: span.clone(),
    })
}

fn is_range_form(se: &Sexp) -> bool {
    if let Sexp::List(items, _) = se {
        if !items.is_empty() {
            if let Some((head, _)) = atom_sym(&items[0]) {
                return head == "range" || head == "range-incl";
            }
        }
    }
    false
}

fn parse_range_expr(se: &Sexp) -> DslResult<RangeExpr> {
    let (items, span) = match se {
        Sexp::List(v, sp) => (v, sp),
        _ => return Err(Diag::new("range form must be a list").with_span(se_span(se))),
    };
    if items.is_empty() {
        return Err(Diag::new("range form is (range start end [step])").with_span(span.clone()));
    }
    let (head, head_sp) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("range head must be a symbol").with_span(se_span(&items[0])))?;
    let inclusive = match head.as_str() {
        "range" => false,
        "range-incl" => true,
        _ => {
            return Err(Diag::new("for expects (range ...) or (range-incl ...)").with_span(head_sp))
        }
    };
    if items.len() < 3 || items.len() > 4 {
        return Err(Diag::new("range form is (range start end [step])").with_span(head_sp));
    }
    let start = parse_expr(&items[1])?;
    let end = parse_expr(&items[2])?;
    let step = if items.len() == 4 {
        Some(Box::new(parse_expr(&items[3])?))
    } else {
        None
    };
    Ok(RangeExpr {
        start: Box::new(start),
        end: Box::new(end),
        step,
        inclusive,
        span: span.clone(),
    })
}

fn parse_let_bindings(se: &Sexp) -> DslResult<Vec<LetBinding>> {
    let items = match se {
        Sexp::Brack(items, _) => items,
        _ => return Err(Diag::new("let bindings must be in [..]").with_span(se_span(se))),
    };
    let mut bindings = Vec::new();
    let mut counter: u32 = 0;

    let all_pairs = items
        .iter()
        .all(|it| matches!(it, Sexp::List(v, _) if v.len() == 2));
    if all_pairs {
        for it in items {
            let (pair, sp) = match it {
                Sexp::List(v, sp) => (v, sp.clone()),
                _ => continue,
            };
            let expr = parse_expr(&pair[1])?;
            let mut out = destructure_binding(&pair[0], expr, &mut counter)?;
            for b in out.iter_mut() {
                b.span = sp.clone();
            }
            bindings.append(&mut out);
        }
        return Ok(bindings);
    }
    if items.len() % 2 != 0 {
        return Err(Diag::new("let bindings must be name/expr pairs").with_span(se_span(se)));
    }
    let mut idx = 0usize;
    while idx < items.len() {
        let expr = parse_expr(&items[idx + 1])?;
        let mut out = destructure_binding(&items[idx], expr, &mut counter)?;
        bindings.append(&mut out);
        idx += 2;
    }
    Ok(bindings)
}

fn destructure_binding(pat: &Sexp, expr: Expr, counter: &mut u32) -> DslResult<Vec<LetBinding>> {
    match pat {
        Sexp::Atom(TokKind::Sym(s), sp) => {
            if s == "_" {
                let tmp = fresh_destruct_name(counter);
                return Ok(vec![LetBinding {
                    name: tmp.clone(),
                    rust_name: rust_value_name(&tmp),
                    ann: None,
                    expr,
                    span: sp.clone(),
                }]);
            }
            let (base, ann) = split_binding_name(s, sp)?;
            Ok(vec![LetBinding {
                name: base.clone(),
                rust_name: rust_value_name(&base),
                ann,
                expr,
                span: sp.clone(),
            }])
        }
        Sexp::Brack(items, sp) | Sexp::List(items, sp) => {
            let tmp = fresh_destruct_name(counter);
            let tmp_rust = rust_value_name(&tmp);
            let mut out = vec![LetBinding {
                name: tmp.clone(),
                rust_name: tmp_rust.clone(),
                ann: None,
                expr,
                span: sp.clone(),
            }];
            for (idx, item) in items.iter().enumerate() {
                let item_sp = se_span(item);
                let get_expr = Expr::Call {
                    op: "darcy.vec/get".to_string(),
                    args: vec![
                        Expr::Var(tmp_rust.clone(), item_sp.clone()),
                        Expr::Int(idx as i64, item_sp.clone()),
                    ],
                    span: item_sp.clone(),
                };
                let mut nested = destructure_binding(item, get_expr, counter)?;
                out.append(&mut nested);
            }
            Ok(out)
        }
        _ => Err(
            Diag::new("let binding name must be a symbol or vector pattern")
                .with_span(se_span(pat)),
        ),
    }
}

fn fresh_destruct_name(counter: &mut u32) -> String {
    let name = format!("destruct-{}", counter);
    *counter += 1;
    name
}

fn split_binding_name(name: &str, sp: &Span) -> DslResult<(String, Option<Ty>)> {
    let mut parts = name.splitn(2, ':');
    let base = parts.next().unwrap().to_string();
    let ann = match parts.next() {
        Some(t) => Some(parse_type_from_sym_checked(t, sp)?),
        None => None,
    };
    let base = ensure_lisp_ident(&base, sp, "binding name")?;
    Ok((base, ann))
}

fn parse_match(span: &Span, items: &[Sexp]) -> DslResult<Expr> {
    if items.len() < 2 {
        return Err(
            Diag::new("case form is (case expr (Variant (...) expr) ...)").with_span(span.clone()),
        );
    }
    let scrutinee = parse_expr(&items[0])?;
    let mut arms = Vec::new();
    for arm in items.iter().skip(1) {
        let (aitems, aspan) = match arm {
            Sexp::List(v, sp) => (v, sp.clone()),
            _ => return Err(Diag::new("case arm must be a list").with_span(se_span(arm))),
        };
        if aitems.len() < 2 {
            return Err(Diag::new("case arm must have a pattern and body").with_span(aspan));
        }
        let (head, head_sp) = atom_sym(&aitems[0]).ok_or_else(|| {
            Diag::new("case arm head must be a symbol").with_span(se_span(&aitems[0]))
        })?;
        if head == "_" {
            if aitems.len() != 2 {
                return Err(Diag::new("wildcard case arm is (_ expr)").with_span(aspan));
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
                            let name = if name == "_" {
                                "_".to_string()
                            } else {
                                let n = ensure_lisp_ident(&name, &nsp, "binding name")?;
                                rust_value_name(&n)
                            };
                            bindings.push((
                                field,
                                name,
                                Span {
                                    start: fsp.start,
                                    end: nsp.end,
                                },
                            ));
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

fn parse_cond(span: &Span, items: &[Sexp]) -> DslResult<Expr> {
    if items.is_empty() {
        return Err(Diag::new("cond form is (cond (test expr) ...)").with_span(span.clone()));
    }
    let mut clauses = Vec::new();
    for clause in items {
        let (citems, cspan) = match clause {
            Sexp::List(v, sp) => (v, sp.clone()),
            _ => return Err(Diag::new("cond clause must be a list").with_span(se_span(clause))),
        };
        if citems.len() != 2 {
            return Err(Diag::new("cond clause must be (test expr)").with_span(cspan));
        }

        let (head, _) =
            atom_sym(&citems[0]).unwrap_or_else(|| ("".to_string(), se_span(&citems[0])));
        let is_else = head == "else";
        let test = if is_else {
            Expr::Bool(true, se_span(&citems[0]))
        } else {
            parse_expr(&citems[0])?
        };
        let body = parse_expr(&citems[1])?;
        clauses.push((test, body, cspan, is_else));
    }
    if clauses.iter().any(|c| c.3) {
        let last_else = clauses.last().map(|c| c.3).unwrap_or(false);
        if !last_else {
            return Err(Diag::new("cond 'else' clause must be last").with_span(span.clone()));
        }
    }
    let mut expr = Expr::Unit(span.clone());
    for (test, body, cspan, is_else) in clauses.into_iter().rev() {
        if is_else {
            expr = body;
        } else {
            expr = Expr::If {
                cond: Box::new(test),
                then_br: Box::new(body),
                else_br: Some(Box::new(expr)),
                span: cspan,
            };
        }
    }
    Ok(expr)
}

pub fn parse_fn(se: &Sexp) -> DslResult<FnDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (defn ... )").with_span(se_span(se))),
    };
    if items.len() < 4 {
        return Err(Diag::new("defn form is (defn name [params] expr ...)").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'defn'").with_span(se_span(&items[0])))?;
    let (mut exported, specialize) = match head.as_str() {
        "defn" => (false, false),
        "defn.specialize" => (false, true),
        "defpub" => (true, false),
        "defpub.specialize" => (true, true),
        _ => {
            return Err(Diag::new("expected 'defn' or 'defpub'").with_span(se_span(&items[0])));
        }
    };
    let (name, name_sp) = atom_sym(&items[1])
        .ok_or_else(|| Diag::new("expected function name").with_span(se_span(&items[1])))?;
    let name = ensure_callable_ident(&name, &name_sp, "function name")?;
    if name == "main" {
        exported = true;
    }
    let params = parse_params(&items[2])?;
    let body = parse_body_expr(&items[3..], &span)?;
    Ok(FnDef {
        name: name.clone(),
        rust_name: rust_value_name(&name),
        params,
        body,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
        specialize,
        exported,
        extern_: false,
        extern_ret: None,
    })
}

fn parse_export(se: &Sexp) -> DslResult<FnDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (export (defn ...))").with_span(se_span(se))),
    };
    if items.len() != 2 {
        return Err(Diag::new("export form is (export (defn ...))").with_span(span));
    }
    let inner = parse_fn(&items[1])?;
    Ok(FnDef {
        exported: true,
        ..inner
    })
}

fn parse_body_expr(items: &[Sexp], span: &Span) -> DslResult<Expr> {
    if items.is_empty() {
        return Err(Diag::new("expected expression").with_span(span.clone()));
    }
    if items.len() == 1 {
        return parse_expr(&items[0]);
    }
    let mut exprs = Vec::new();
    for item in items {
        exprs.push(parse_expr(item)?);
    }
    Ok(Expr::Do {
        exprs,
        span: span.clone(),
    })
}

pub fn parse_inline(se: &Sexp) -> DslResult<InlineDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (defin ...)").with_span(se_span(se))),
    };
    if items.len() < 4 {
        return Err(Diag::new("defin form is (defin name [params] expr ...)").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'defin'").with_span(se_span(&items[0])))?;
    if head != "defin" {
        return Err(Diag::new("expected 'defin'").with_span(se_span(&items[0])));
    }
    let (name, name_sp) = atom_sym(&items[1])
        .ok_or_else(|| Diag::new("expected inline function name").with_span(se_span(&items[1])))?;
    let name = ensure_callable_ident(&name, &name_sp, "inline function name")?;
    let params = parse_params(&items[2])?;
    let body = parse_body_expr(&items[3..], &span)?;
    Ok(InlineDef {
        name: name.clone(),
        rust_name: rust_value_name(&name),
        params,
        body,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
    })
}

fn union_variant_name(ty: &str) -> String {
    ty.rsplit_once('/')
        .map(|(_, tail)| tail)
        .unwrap_or(ty)
        .to_string()
}

pub fn parse_union_alias(se: &Sexp) -> DslResult<UnionDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (defunion ...)").with_span(se_span(se))),
    };
    if items.len() < 3 {
        return Err(Diag::new("defunion requires a name and at least one type").with_span(span));
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
    for item in &items[2..] {
        let (ty_sym, ty_sp) = atom_sym(item)
            .ok_or_else(|| Diag::new("expected type name").with_span(se_span(item)))?;
        let ty = parse_type_from_sym_checked(&ty_sym, &ty_sp)?;
        let ty_name = match &ty {
            Ty::Named(n) => n.clone(),
            _ => {
                return Err(Diag::new("defunion only accepts named types").with_span(ty_sp.clone()))
            }
        };
        let vname = union_variant_name(&ty_name);
        let rust_vname = rust_type_name(&vname);
        let field = Field {
            name: "value".to_string(),
            rust_name: "value".to_string(),
            ty,
            span: ty_sp.clone(),
        };
        variants.push(VariantDef {
            name: vname,
            rust_name: rust_vname,
            fields: vec![field],
            span: ty_sp,
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

pub fn parse_def(se: &Sexp) -> DslResult<Def> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (def ...)").with_span(se_span(se))),
    };
    if items.len() != 3 {
        return Err(Diag::new("def form is (def name expr)").with_span(span));
    }
    let (head, _) = atom_sym(&items[0])
        .ok_or_else(|| Diag::new("expected symbol 'def'").with_span(se_span(&items[0])))?;
    if head != "def" {
        return Err(Diag::new("expected 'def'").with_span(se_span(&items[0])));
    }
    let (name, name_sp) = atom_sym(&items[1])
        .ok_or_else(|| Diag::new("expected def name").with_span(se_span(&items[1])))?;
    let (base, ann) = split_binding_name(&name, &name_sp)?;
    let expr = parse_expr(&items[2])?;
    Ok(Def {
        name: base.clone(),
        rust_name: rust_value_name(&base),
        ann,
        expr,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
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

    let form = items
        .get(idx)
        .ok_or_else(|| Diag::new("extern requires a form").with_span(span.clone()))?;
    let (fitems, _fspan) = match form {
        Sexp::List(v, sp) => (v, sp.clone()),
        _ => return Err(Diag::new("extern requires a def form").with_span(se_span(form))),
    };
    if fitems.is_empty() {
        return Err(Diag::new("extern requires a def form").with_span(se_span(form)));
    }

    let (head, _) = atom_sym(&fitems[0]).ok_or_else(|| {
        Diag::new("extern form head must be a symbol").with_span(se_span(&fitems[0]))
    })?;
    match head.as_str() {
        "defrecord" => {
            let mut sd = parse_struct(form)?;
            sd.extern_ = true;
            if let Some(rust) = override_name {
                sd.rust_name = rust;
            }
            Ok(Top::Struct(sd))
        }
        "defenum" => {
            let mut ud = parse_union(form)?;
            ud.extern_ = true;
            if let Some(rust) = override_name {
                ud.rust_name = rust;
            }
            Ok(Top::Union(ud))
        }
        "defn" => parse_extern_fn(form, override_name),
        _ => {
            Err(Diag::new("extern can wrap defrecord, defenum, or defn")
                .with_span(se_span(&fitems[0])))
        }
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
    let name = ensure_callable_ident_allow_reserved(&name, &name_sp, "function name")?;
    let params = parse_params(&items[2])?;

    let (ret_sym, ret_sp) = atom_sym(&items[3]).ok_or_else(|| {
        Diag::new("extern defn return type must be a symbol").with_span(se_span(&items[3]))
    })?;
    let ret_ty = parse_type_from_sym_checked(&ret_sym, &ret_sp)?;
    let _dummy_body = Expr::Var("__extern".to_string(), ret_sp.clone());
    let rust_name = override_name.unwrap_or_else(|| rust_value_name(&name));
    Ok(Top::Func(FnDef {
        name,
        rust_name,
        params,
        body: Expr::Unit(span.clone()),
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
        specialize: false,
        exported: false,
        extern_: true,
        extern_ret: Some(ret_ty),
    }))
}

fn se_span(se: &Sexp) -> Span {
    match se {
        Sexp::Atom(_, sp) => sp.clone(),
        Sexp::List(_, sp) => sp.clone(),
        Sexp::Brack(_, sp) => sp.clone(),
        Sexp::Brace(_, sp) => sp.clone(),
        Sexp::Set(_, sp) => sp.clone(),
    }
}

pub fn parse_toplevel(sexps: &[Sexp]) -> DslResult<Vec<Top>> {
    let mut out = Vec::new();
    for se in sexps {
        let (items, span) = match se {
            Sexp::List(items, span) => (items, span.clone()),
            _ => return Err(Diag::new("top-level forms must be lists").with_span(se_span(se))),
        };
        if items.is_empty() {
            return Err(Diag::new("empty top-level list").with_span(se_span(se)));
        }

        let (head, _) = atom_sym(&items[0]).ok_or_else(|| {
            Diag::new("top-level head must be a symbol").with_span(se_span(&items[0]))
        })?;
        match head.as_str() {
            "require" => {
                for decl in parse_use_decl(&items[1..], &span)? {
                    out.push(Top::Use(decl));
                }
            }
            "extern" => out.push(parse_extern_toplevel(se)?),
            "defrecord" => out.push(Top::Struct(parse_struct(se)?)),
            "defenum" => out.push(Top::Union(parse_union(se)?)),
            "defunion" => out.push(Top::Union(parse_union_alias(se)?)),
            "defn" | "defn.specialize" | "defpub" | "defpub.specialize" => {
                out.push(Top::Func(parse_fn(se)?))
            }
            "export" => out.push(Top::Func(parse_export(se)?)),
            "defin" => out.push(Top::Inline(parse_inline(se)?)),
            "def" => out.push(Top::Def(parse_def(se)?)),
            _ => {
                return Err(Diag::new(format!("unknown top-level form '{}'", head))
                    .with_span(se_span(&items[0])));
            }
        }
    }
    Ok(out)
}

pub fn parse_use_decl(items: &[Sexp], span: &Span) -> DslResult<Vec<UseDecl>> {
    if items.is_empty() {
        return Err(
            Diag::new("require form is (require [path :as name :refer [a b]])")
                .with_span(span.clone()),
        );
    }
    let mut out = Vec::new();
    for spec in items {
        if !matches!(spec, Sexp::Brack(_, _)) {
            return Err(
                Diag::new("require specs must be vectors like [path :as name]")
                    .with_span(se_span(spec)),
            );
        }
        out.push(parse_use_spec(spec, span)?);
    }
    Ok(out)
}

fn parse_use_spec(spec: &Sexp, _span: &Span) -> DslResult<UseDecl> {
    let (items, sp) = match spec {
        Sexp::Brack(items, sp) => (items, sp.clone()),
        _ => {
            return Err(
                Diag::new("require spec must be [path :as name :refer [a b]]")
                    .with_span(se_span(spec)),
            )
        }
    };
    if items.is_empty() {
        return Err(Diag::new("require spec must include a path").with_span(sp));
    }
    let (path, path_sp) = match &items[0] {
        Sexp::Atom(TokKind::Sym(s), sp) => (s.clone(), sp.clone()),
        _ => return Err(Diag::new("require path must be a symbol").with_span(sp)),
    };
    let mut alias = None;
    let mut only = None;
    let mut open = false;
    let mut idx = 1usize;
    while idx < items.len() {
        let key = match &items[idx] {
            Sexp::Atom(TokKind::Sym(s), _) => s.clone(),
            _ => return Err(Diag::new("require modifier must be a symbol").with_span(sp.clone())),
        };
        idx += 1;
        match key.as_str() {
            ":as" => {
                let (name, nsp) = match items.get(idx) {
                    Some(Sexp::Atom(TokKind::Sym(s), sp)) => (s.clone(), sp.clone()),
                    _ => {
                        return Err(Diag::new("require :as must be followed by a symbol")
                            .with_span(sp.clone()))
                    }
                };
                let name = ensure_lisp_ident(&name, &nsp, "require alias")?;
                alias = Some(name);
                idx += 1;
            }
            ":refer" => {
                let next = items.get(idx).ok_or_else(|| {
                    Diag::new("require :refer must be followed by a list or :all")
                        .with_span(sp.clone())
                })?;
                match next {
                    Sexp::Atom(TokKind::Sym(s), _) if s == ":all" || s == "all" => {
                        open = true;
                        idx += 1;
                    }
                    Sexp::List(v, _) | Sexp::Brack(v, _) => {
                        let mut names = Vec::new();
                        for it in v {
                            if let Some((s, sp)) = atom_sym(it) {
                                let s = ensure_lisp_ident(&s, &sp, "require :refer name")?;
                                names.push(s);
                            } else {
                                return Err(Diag::new("require :refer entries must be symbols")
                                    .with_span(se_span(it)));
                            }
                        }
                        only = Some(names);
                        idx += 1;
                    }
                    _ => {
                        return Err(
                            Diag::new("require :refer must be followed by a list or :all")
                                .with_span(se_span(next)),
                        )
                    }
                }
            }
            _ => return Err(Diag::new("unknown require modifier").with_span(sp.clone())),
        }
    }
    if open && only.is_some() {
        return Err(Diag::new("require cannot combine :refer :all with :refer list").with_span(sp));
    }
    Ok(UseDecl {
        path,
        alias,
        only,
        open,
        span: path_sp,
    })
}

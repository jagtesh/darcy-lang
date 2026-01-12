use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;

#[derive(Debug, Clone, Copy)]
struct Loc {
    line: usize,
    col: usize,
    byte: usize,
}

#[derive(Debug, Clone)]
struct Span {
    start: Loc,
    end: Loc,
}

#[derive(Debug, Clone)]
struct Diag {
    message: String,
    span: Option<Span>,
}

impl Diag {
    fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), span: None }
    }
    fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

type DslResult<T> = Result<T, Diag>;

#[derive(Debug, Clone, PartialEq)]
enum TokKind {
    LParen, RParen,
    LBrack, RBrack,
    Sym(String),
    Int(i64),
    Float(f64),
}

#[derive(Debug, Clone)]
struct Tok {
    kind: TokKind,
    span: Span,
}

fn is_sym_start(c: char) -> bool {
    c.is_ascii_alphabetic() || "_+-*/<>=!?".contains(c)
}
fn is_sym_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_+-*/<>=!?.".contains(c) || c == ':' || c == '/'
}

fn lex(input: &str) -> DslResult<Vec<Tok>> {
    let mut toks = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    let bytes = input.as_bytes();
    while i < bytes.len() {
        let c = input[i..].chars().next().unwrap();
        let start = Loc { line, col, byte: i };

        // whitespace + comments
        if c == ';' {
            // comment to end of line
            while i < bytes.len() {
                let ch = input[i..].chars().next().unwrap();
                if ch == '\n' { break; }
                i += ch.len_utf8();
                col += 1;
            }
            continue;
        }
        if c.is_whitespace() {
            if c == '\n' {
                i += 1;
                line += 1;
                col = 1;
            } else {
                i += c.len_utf8();
                col += 1;
            }
            continue;
        }

        // single-char tokens
        let kind = match c {
            '(' => { i += 1; col += 1; TokKind::LParen }
            ')' => { i += 1; col += 1; TokKind::RParen }
            '[' => { i += 1; col += 1; TokKind::LBrack }
            ']' => { i += 1; col += 1; TokKind::RBrack }
            _ => {
                // number?
                if c.is_ascii_digit() || (c == '-' && input[i+1..].chars().next().map(|x| x.is_ascii_digit()).unwrap_or(false)) {
                    let mut j = i;
                    let mut saw_dot = false;
                    let mut saw_exp = false;
                    let mut first = true;
                    while j < bytes.len() {
                        let ch = input[j..].chars().next().unwrap();
                        if first && ch == '-' {
                            // ok
                        } else if ch.is_ascii_digit() {
                            // ok
                        } else if ch == '.' && !saw_dot && !saw_exp {
                            saw_dot = true;
                        } else if (ch == 'e' || ch == 'E') && !saw_exp {
                            saw_exp = true;
                            // allow exponent sign next
                        } else if saw_exp && (ch == '+' || ch == '-') {
                            // only right after e/E
                        } else {
                            break;
                        }
                        first = false;
                        j += ch.len_utf8();
                    }
                    let s = &input[i..j];
                    i = j;
                    col += s.chars().count();
                    if saw_dot || saw_exp {
                        let v: f64 = s.parse().map_err(|_| Diag::new(format!("invalid float literal: {}", s)).with_span(Span{start, end: Loc{line, col, byte: i}}))?;
                        TokKind::Float(v)
                    } else {
                        let v: i64 = s.parse().map_err(|_| Diag::new(format!("invalid int literal: {}", s)).with_span(Span{start, end: Loc{line, col, byte: i}}))?;
                        TokKind::Int(v)
                    }
                } else if is_sym_start(c) {
                    let mut j = i;
                    while j < bytes.len() {
                        let ch = input[j..].chars().next().unwrap();
                        if is_sym_char(ch) {
                            j += ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let s = &input[i..j];
                    i = j;
                    col += s.chars().count();
                    TokKind::Sym(s.to_string())
                } else {
                    return Err(Diag::new(format!("unexpected character: '{}'", c))
                        .with_span(Span{start, end: Loc{line, col, byte: i + c.len_utf8()}}));
                }
            }
        };

        let end = Loc { line, col, byte: i };
        toks.push(Tok { kind, span: Span { start, end } });
    }

    Ok(toks)
}

#[derive(Debug, Clone)]
enum Sexp {
    Atom(TokKind, Span),
    List(Vec<Sexp>, Span),
    Brack(Vec<Sexp>, Span),
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn new(toks: Vec<Tok>) -> Self { Self { toks, pos: 0 } }
    fn peek(&self) -> Option<&Tok> { self.toks.get(self.pos) }
    fn bump(&mut self) -> Option<Tok> { let t = self.toks.get(self.pos).cloned(); if t.is_some() { self.pos += 1; } t }
    fn expect(&mut self, k: TokKind) -> DslResult<Tok> {
        let t = self.bump().ok_or_else(|| Diag::new("unexpected end of input"))?;
        if std::mem::discriminant(&t.kind) == std::mem::discriminant(&k) {
            Ok(t)
        } else {
            Err(Diag::new(format!("expected {:?}, got {:?}", k, t.kind)).with_span(t.span))
        }
    }

    fn parse_all(&mut self) -> DslResult<Vec<Sexp>> {
        let mut out = Vec::new();
        while self.peek().is_some() {
            out.push(self.parse_one()?);
        }
        Ok(out)
    }

    fn parse_one(&mut self) -> DslResult<Sexp> {
        let t = self.peek().ok_or_else(|| Diag::new("unexpected end of input"))?.clone();
        match &t.kind {
            TokKind::LParen => self.parse_list(),
            TokKind::LBrack => self.parse_brack(),
            TokKind::RParen | TokKind::RBrack => {
                Err(Diag::new("unexpected closing delimiter").with_span(t.span))
            }
            _ => {
                let t = self.bump().unwrap();
                Ok(Sexp::Atom(t.kind, t.span))
            }
        }
    }

    fn parse_list(&mut self) -> DslResult<Sexp> {
        let open = self.expect(TokKind::LParen)?;
        let mut items = Vec::new();
        while let Some(t) = self.peek() {
            match t.kind {
                TokKind::RParen => {
                    let close = self.bump().unwrap();
                    let span = Span { start: open.span.start, end: close.span.end };
                    return Ok(Sexp::List(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '('").with_span(open.span))
    }

    fn parse_brack(&mut self) -> DslResult<Sexp> {
        let open = self.expect(TokKind::LBrack)?;
        let mut items = Vec::new();
        while let Some(t) = self.peek() {
            match t.kind {
                TokKind::RBrack => {
                    let close = self.bump().unwrap();
                    let span = Span { start: open.span.start, end: close.span.end };
                    return Ok(Sexp::Brack(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '['").with_span(open.span))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ty {
    Named(String),
    Unknown,
}

impl Ty {
    fn rust(&self) -> String {
        match self {
            Ty::Named(s) => s.clone(),
            Ty::Unknown => "_".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    ty: Ty,
    span: Span,
}

#[derive(Debug, Clone)]
struct StructDef {
    name: String,
    fields: Vec<Field>,
    span: Span,
}

#[derive(Debug, Clone)]
struct Param {
    name: String,
    ann: Option<Ty>,
    span: Span,
}

#[derive(Debug, Clone)]
enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    Var(String, Span),
    Field { base: Box<Expr>, field: String, span: Span },
    Call { op: String, args: Vec<Expr>, span: Span },
}

impl Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Int(_, s) => s.clone(),
            Expr::Float(_, s) => s.clone(),
            Expr::Var(_, s) => s.clone(),
            Expr::Field { span, .. } => span.clone(),
            Expr::Call { span, .. } => span.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct FnDef {
    name: String,
    params: Vec<Param>,
    body: Expr,
    span: Span,
}

#[derive(Debug, Clone)]
enum Top {
    Struct(StructDef),
    Func(FnDef),
}

fn atom_sym(se: &Sexp) -> Option<(String, Span)> {
    match se {
        Sexp::Atom(TokKind::Sym(s), sp) => Some((s.clone(), sp.clone())),
        _ => None,
    }
}

fn parse_type_from_sym(s: &str) -> Ty {
    Ty::Named(s.to_string())
}

fn parse_struct(se: &Sexp) -> DslResult<StructDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (defstruct ...)").with_span(se_span(se))),
    };
    if items.len() < 2 { return Err(Diag::new("defstruct requires a name").with_span(span)); }
    let (head, _) = atom_sym(&items[0]).ok_or_else(|| Diag::new("expected symbol 'defstruct'").with_span(se_span(&items[0])))?;
    if head != "defstruct" { return Err(Diag::new("expected 'defstruct'").with_span(se_span(&items[0]))); }
    let (name, name_sp) = atom_sym(&items[1]).ok_or_else(|| Diag::new("expected struct name").with_span(se_span(&items[1])))?;

    let mut fields = Vec::new();
    for f in items.iter().skip(2) {
        let (fitems, fspan) = match f {
            Sexp::List(v, sp) => (v, sp.clone()),
            _ => return Err(Diag::new("field must be (name Type)").with_span(se_span(f))),
        };
        if fitems.len() != 2 {
            return Err(Diag::new("field must be (name Type)").with_span(fspan));
        }
        let (fname, fsp) = atom_sym(&fitems[0]).ok_or_else(|| Diag::new("expected field name").with_span(se_span(&fitems[0])))?;
        let (fty_s, _) = atom_sym(&fitems[1]).ok_or_else(|| Diag::new("expected field type").with_span(se_span(&fitems[1])))?;
        fields.push(Field { name: fname, ty: parse_type_from_sym(&fty_s), span: Span{ start: fsp.start, end: fspan.end } });
    }

    Ok(StructDef { name, fields, span: Span{ start: name_sp.start, end: span.end } })
}

fn parse_params(se: &Sexp) -> DslResult<Vec<Param>> {
    let (items, span) = match se {
        Sexp::Brack(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected parameter list in [..]").with_span(se_span(se))),
    };
    let mut out = Vec::new();
    for it in items {
        let (sym, sp) = atom_sym(it).ok_or_else(|| Diag::new("parameter must be a symbol like o:Order").with_span(se_span(it)))?;
        let mut parts = sym.splitn(2, ':');
        let name = parts.next().unwrap().to_string();
        let ann = parts.next().map(|t| parse_type_from_sym(t));
        if name.is_empty() {
            return Err(Diag::new("invalid parameter name").with_span(sp));
        }
        out.push(Param { name, ann, span: sp });
    }
    Ok(out)
}


fn parse_expr(se: &Sexp) -> DslResult<Expr> {
    match se {
        Sexp::Atom(TokKind::Int(v), sp) => Ok(Expr::Int(*v, sp.clone())),
        Sexp::Atom(TokKind::Float(v), sp) => Ok(Expr::Float(*v, sp.clone())),
        Sexp::Atom(TokKind::Sym(s), sp) => {
            // field access sugar: a.b
            if let Some((a, b)) = s.split_once('.') {
                if !a.is_empty() && !b.is_empty() {
                    return Ok(Expr::Field {
                        base: Box::new(Expr::Var(a.to_string(), sp.clone())),
                        field: b.to_string(),
                        span: sp.clone(),
                    });
                }
            }
            Ok(Expr::Var(s.clone(), sp.clone()))
        }
        Sexp::Atom(k, sp) => Err(
            Diag::new(format!("unexpected token {:?} where expression expected", k))
                .with_span(sp.clone())
        ),
        Sexp::List(items, span) => {
            if items.is_empty() {
                return Err(Diag::new("empty list is not a valid expression").with_span(span.clone()));
            }
            let (op, _) = atom_sym(&items[0])
                .ok_or_else(|| Diag::new("call head must be a symbol").with_span(se_span(&items[0])))?;
            let mut args = Vec::new();
            for a in items.iter().skip(1) {
                args.push(parse_expr(a)?);
            }
            Ok(Expr::Call { op, args, span: span.clone() })
        }
        Sexp::Brack(_, span) => Err(Diag::new("unexpected [..] where expression expected").with_span(span.clone())),
    }
}

fn parse_fn(se: &Sexp) -> DslResult<FnDef> {
    let (items, span) = match se {
        Sexp::List(items, span) => (items, span.clone()),
        _ => return Err(Diag::new("expected (defn ...)").with_span(se_span(se))),
    };
    if items.len() != 4 {
        return Err(Diag::new("defn form is (defn name [params] body)").with_span(span));
    }
    let (head, _) = atom_sym(&items[0]).ok_or_else(|| Diag::new("expected symbol 'defn'").with_span(se_span(&items[0])))?;
    if head != "defn" { return Err(Diag::new("expected 'defn'").with_span(se_span(&items[0]))); }
    let (name, name_sp) = atom_sym(&items[1]).ok_or_else(|| Diag::new("expected function name").with_span(se_span(&items[1])))?;
    let params = parse_params(&items[2])?;
    let body = parse_expr(&items[3])?;
    Ok(FnDef { name, params, body, span: Span{ start: name_sp.start, end: span.end } })
}

fn se_span(se: &Sexp) -> Span {
    match se {
        Sexp::Atom(_, sp) => sp.clone(),
        Sexp::List(_, sp) => sp.clone(),
        Sexp::Brack(_, sp) => sp.clone(),
    }
}

fn parse_toplevel(sexps: &[Sexp]) -> DslResult<Vec<Top>> {
    let mut out = Vec::new();
    for se in sexps {
        let (items, _) = match se {
            Sexp::List(items, span) => (items, span.clone()),
            _ => return Err(Diag::new("top-level forms must be lists").with_span(se_span(se))),
        };
        if items.is_empty() {
            return Err(Diag::new("empty top-level list").with_span(se_span(se)));
        }
        let (head, _) = atom_sym(&items[0]).ok_or_else(|| Diag::new("top-level head must be a symbol").with_span(se_span(&items[0])))?;
        match head.as_str() {
            "defstruct" => out.push(Top::Struct(parse_struct(se)?)),
            "defn" => out.push(Top::Func(parse_fn(se)?)),
            _ => return Err(Diag::new(format!("unknown top-level form '{}'", head)).with_span(se_span(&items[0]))),
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
struct TypeEnv {
    structs: BTreeMap<String, StructDef>,
}

impl TypeEnv {
    fn new() -> Self { Self { structs: BTreeMap::new() } }
    fn insert_struct(&mut self, sd: StructDef) -> DslResult<()> {
        if self.structs.contains_key(&sd.name) {
            return Err(Diag::new(format!("duplicate struct '{}'", sd.name)).with_span(sd.span));
        }
        self.structs.insert(sd.name.clone(), sd);
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct FnSig {
    params: BTreeMap<String, Ty>,
    ret: Ty,
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
            for a in args { collect_field_constraints(a, out); }
        }
        _ => {}
    }
}

fn infer_param_types(env: &TypeEnv, f: &FnDef) -> DslResult<BTreeMap<String, Ty>> {
    let mut param_tys: BTreeMap<String, Ty> = BTreeMap::new();
    let mut param_spans: BTreeMap<String, Span> = BTreeMap::new();

    for p in &f.params {
        if param_tys.contains_key(&p.name) {
            return Err(Diag::new(format!("duplicate parameter '{}'", p.name)).with_span(p.span.clone()));
        }
        param_tys.insert(p.name.clone(), p.ann.clone().unwrap_or(Ty::Unknown));
        param_spans.insert(p.name.clone(), p.span.clone());
    }

    // gather constraints: param -> set(field names)
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
        let cur = param_tys.get(&p).cloned().unwrap_or(Ty::Unknown);
        if let Ty::Named(_) = cur {
            // validate fields exist
            let ty_name = match &cur { Ty::Named(s) => s.clone(), _ => unreachable!() };
            let sd = env.structs.get(&ty_name).ok_or_else(|| Diag::new(format!("unknown type '{}'", ty_name)).with_span(param_spans[&p].clone()))?;
            for fld in &fields {
                if !sd.fields.iter().any(|ff| ff.name == *fld) {
                    return Err(Diag::new(format!("type '{}' has no field '{}'", ty_name, fld)).with_span(param_spans[&p].clone()));
                }
            }
            continue;
        }

        // infer by structural match across known structs
        let mut candidates = Vec::new();
        for (name, sd) in &env.structs {
            let ok = fields.iter().all(|fld| sd.fields.iter().any(|ff| &ff.name == fld));
            if ok { candidates.push(name.clone()); }
        }

        if candidates.is_empty() {
            return Err(Diag::new(format!(
                "cannot infer type for '{}': no known struct contains fields {}",
                p,
                fmt_set(&fields)
            )).with_span(param_spans[&p].clone()));
        }
        if candidates.len() > 1 {
            return Err(Diag::new(format!(
                "ambiguous type for '{}': matches {}. Add annotation like {}:{}",
                p,
                candidates.join(", "),
                p,
                candidates[0],
            )).with_span(param_spans[&p].clone()));
        }
        param_tys.insert(p, Ty::Named(candidates[0].clone()));
    }

    // any remaining unknown params without constraints -> error (we want Elm-like clarity)
    for (p, ty) in param_tys.clone() {
        if ty == Ty::Unknown {
            return Err(Diag::new(format!(
                "cannot infer type for parameter '{}': no constraints. Add annotation like {}:Type",
                p, p
            )).with_span(param_spans[&p].clone()));
        }
    }

    Ok(param_tys)
}

fn fmt_set(s: &BTreeSet<String>) -> String {
    let mut v: Vec<_> = s.iter().cloned().collect();
    v.sort();
    format!("{{{}}}", v.join(", "))
}

#[derive(Debug, Clone)]
struct TypedExpr {
    expr: Expr,
    ty: Ty,
    casts: Vec<CastHint>,
}

#[derive(Debug, Clone)]
struct CastHint {
    // if this span matches a sub-expression, cast it to target
    span: Span,
    target: Ty,
}

fn infer_expr_type(env: &TypeEnv, vars: &BTreeMap<String, Ty>, e: &Expr) -> DslResult<TypedExpr> {
    match e {
        Expr::Int(_, sp) => Ok(TypedExpr { expr: e.clone(), ty: Ty::Named("i32".to_string()), casts: vec![] }),
        Expr::Float(_, sp) => Ok(TypedExpr { expr: e.clone(), ty: Ty::Named("f64".to_string()), casts: vec![] }),
        Expr::Var(v, sp) => {
            let ty = vars.get(v).cloned().ok_or_else(|| Diag::new(format!("unknown variable '{}'", v)).with_span(sp.clone()))?;
            Ok(TypedExpr { expr: e.clone(), ty, casts: vec![] })
        }
        Expr::Field { base, field, span } => {
            let tb = infer_expr_type(env, vars, base)?;
            let base_ty = tb.ty.clone();
            let struct_name = match base_ty {
                Ty::Named(n) => n,
                Ty::Unknown => return Err(Diag::new("cannot access field on unknown type").with_span(span.clone())),
            };
            let sd = env.structs.get(&struct_name)
                .ok_or_else(|| Diag::new(format!("unknown type '{}'", struct_name)).with_span(span.clone()))?;
            let f = sd.fields.iter().find(|ff| ff.name == *field)
                .ok_or_else(|| Diag::new(format!("type '{}' has no field '{}'", struct_name, field)).with_span(span.clone()))?;
            Ok(TypedExpr { expr: e.clone(), ty: f.ty.clone(), casts: tb.casts })
        }
        Expr::Call { op, args, span } => {
            // infer args
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
                        return Err(Diag::new(format!("'{}' expects 2 arguments", op)).with_span(span.clone()));
                    }
                    let a = &targs[0];
                    let b = &targs[1];
                    let (out_ty, extra_casts) = numeric_binop(&a.ty, &b.ty, &a.expr.span(), &b.expr.span())
                        .map_err(|m| Diag::new(m).with_span(span.clone()))?;
                    casts.extend(extra_casts);
                    Ok(TypedExpr { expr: e.clone(), ty: out_ty, casts })
                }
                _ => Err(Diag::new(format!("unknown operator '{}'", op)).with_span(span.clone())),
            }
        }
    }
}

fn numeric_binop(a: &Ty, b: &Ty, a_sp: &Span, b_sp: &Span) -> Result<(Ty, Vec<CastHint>), String> {
    let ai = a == &Ty::Named("i32".to_string()) || a == &Ty::Named("i64".to_string()) || a == &Ty::Named("u32".to_string()) || a == &Ty::Named("u64".to_string());
    let af = a == &Ty::Named("f32".to_string()) || a == &Ty::Named("f64".to_string());
    let bi = b == &Ty::Named("i32".to_string()) || b == &Ty::Named("i64".to_string()) || b == &Ty::Named("u32".to_string()) || b == &Ty::Named("u64".to_string());
    let bf = b == &Ty::Named("f32".to_string()) || b == &Ty::Named("f64".to_string());

    if !(ai || af) || !(bi || bf) {
        return Err(format!("operator expects numeric types, got '{}' and '{}'", a.rust(), b.rust()));
    }

    // simple promotion: if either is f64, cast ints to f64 and result f64.
    if a == &Ty::Named("f64".to_string()) || b == &Ty::Named("f64".to_string()) {
        let mut casts = Vec::new();
        if ai && a != &Ty::Named("f64".to_string()) {
            casts.push(CastHint { span: a_sp.clone(), target: Ty::Named("f64".to_string()) });
        }
        if bi && b != &Ty::Named("f64".to_string()) {
            casts.push(CastHint { span: b_sp.clone(), target: Ty::Named("f64".to_string()) });
        }
        return Ok((Ty::Named("f64".to_string()), casts));
    }

    // otherwise if either is float, use f32
    if af || bf {
        let mut casts = Vec::new();
        if ai && a != &Ty::Named("f32".to_string()) {
            casts.push(CastHint { span: a_sp.clone(), target: Ty::Named("f32".to_string()) });
        }
        if bi && b != &Ty::Named("f32".to_string()) {
            casts.push(CastHint { span: b_sp.clone(), target: Ty::Named("f32".to_string()) });
        }
        return Ok((Ty::Named("f32".to_string()), casts));
    }

    // ints: keep i32 by default
    Ok((Ty::Named("i32".to_string()), vec![]))
}

fn lower(env: &TypeEnv, tops: &[Top]) -> DslResult<String> {
    let mut out = String::new();
    out.push_str("// Generated by dslc (MVP)\n");
    out.push_str("#![allow(dead_code)]\n\n");

    // structs
    for t in tops {
        if let Top::Struct(sd) = t {
            out.push_str(&format!("pub struct {} {{\n", sd.name));
            for f in &sd.fields {
                out.push_str(&format!("    pub {}: {},\n", f.name, f.ty.rust()));
            }
            out.push_str("}\n\n");
        }
    }

    // functions
    for t in tops {
        if let Top::Func(fd) = t {
            let param_tys = infer_param_types(env, fd)?;
            let mut vars = BTreeMap::new();
            for (k, v) in &param_tys { vars.insert(k.clone(), v.clone()); }

            let texpr = infer_expr_type(env, &vars, &fd.body)?;

            out.push_str(&format!("pub fn {}(", fd.name));
            let mut first = true;
            for p in &fd.params {
                if !first { out.push_str(", "); }
                first = false;
                let ty = param_tys.get(&p.name).unwrap();
                out.push_str(&format!("{}: {}", p.name, ty.rust()));
            }
            out.push_str(&format!(") -> {} {{\n", texpr.ty.rust()));
            out.push_str("    ");
            out.push_str(&lower_expr(&fd.body, &texpr.casts));
            out.push_str("\n}\n\n");
        }
    }

    Ok(out)
}

fn lower_expr(e: &Expr, casts: &[CastHint]) -> String {
    // if there is a cast hint matching this expr span, wrap
    let mut inner = match e {
        Expr::Int(v, _) => v.to_string(),
        Expr::Float(v, _) => {
            // ensure Rust float literal
            let mut s = v.to_string();
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                s.push_str(".0");
            }
            s
        }
        Expr::Var(v, _) => v.clone(),
        Expr::Field { base, field, .. } => format!("{}.{}", lower_expr(base, casts), field),
        Expr::Call { op, args, .. } => {
            if args.len() == 2 && ["+","-","*","/"].contains(&op.as_str()) {
                format!("({} {} {})", lower_expr(&args[0], casts), op, lower_expr(&args[1], casts))
            } else {
                format!("/* unsupported call {} */", op)
            }
        }
    };

    for ch in casts {
        if spans_eq(&e.span(), &ch.span) {
            inner = format!("({} as {})", inner, ch.target.rust());
            break;
        }
    }
    inner
}

fn spans_eq(a: &Span, b: &Span) -> bool {
    a.start.byte == b.start.byte && a.end.byte == b.end.byte
}

fn render_diag(file: &str, src: &str, d: &Diag) -> String {
    let mut s = String::new();
    s.push_str(&format!("error: {}\n", d.message));
    if let Some(sp) = &d.span {
        s.push_str(&format!(" --> {}:{}:{}\n", file, sp.start.line, sp.start.col));
        // include source line
        if let Some(line) = src.lines().nth(sp.start.line.saturating_sub(1)) {
            s.push_str(&format!("  |\n{:>2} | {}\n  |", sp.start.line, line));
            // caret under start col
            let caret_pos = sp.start.col.saturating_sub(1);
            s.push_str(&format!("\n  | {}^\n", " ".repeat(caret_pos)));
        }
    }
    s
}

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        eprintln!("dslc (MVP)\n\nUsage:\n  dslc <input.dsl>\n\nOutputs Rust to stdout.\n");
        std::process::exit(2);
    }
    let file = args.remove(0);
    let src = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file, e);
            std::process::exit(1);
        }
    };

    let res = (|| -> DslResult<String> {
        let toks = lex(&src)?;
        let mut p = Parser::new(toks);
        let sexps = p.parse_all()?;
        let tops = parse_toplevel(&sexps)?;

        let mut env = TypeEnv::new();
        for t in &tops {
            if let Top::Struct(sd) = t {
                env.insert_struct(sd.clone())?;
            }
        }

        lower(&env, &tops)
    })();

    match res {
        Ok(rust) => {
            print!("{}", rust);
        }
        Err(d) => {
            eprintln!("{}", render_diag(&file, &src, &d));
            std::process::exit(1);
        }
    }
}

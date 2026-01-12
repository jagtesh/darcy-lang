use crate::diag::{Diag, DslResult, Span};
use crate::lexer::TokKind;
use crate::parser::Sexp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Named(String),
    Unknown,
}

impl Ty {
    pub fn rust(&self) -> String {
        match self {
            Ty::Named(s) => s.clone(),
            Ty::Unknown => "_".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ann: Option<Ty>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    Var(String, Span),
    Field {
        base: Box<Expr>,
        field: String,
        span: Span,
    },
    Call {
        op: String,
        args: Vec<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
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
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Top {
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
        let (fty_s, _) = atom_sym(&fitems[1])
            .ok_or_else(|| Diag::new("expected field type").with_span(se_span(&fitems[1])))?;
        fields.push(Field {
            name: fname,
            ty: parse_type_from_sym(&fty_s),
            span: Span {
                start: fsp.start,
                end: fspan.end,
            },
        });
    }

    Ok(StructDef {
        name,
        fields,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
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
        let ann = parts.next().map(|t| parse_type_from_sym(t));
        if name.is_empty() {
            return Err(Diag::new("invalid parameter name").with_span(sp));
        }
        out.push(Param { name, ann, span: sp });
    }
    Ok(out)
}

pub fn parse_expr(se: &Sexp) -> DslResult<Expr> {
    match se {
        Sexp::Atom(TokKind::Int(v), sp) => Ok(Expr::Int(*v, sp.clone())),
        Sexp::Atom(TokKind::Float(v), sp) => Ok(Expr::Float(*v, sp.clone())),
        Sexp::Atom(TokKind::Sym(s), sp) => {
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
                .with_span(sp.clone()),
        ),
        Sexp::List(items, span) => {
            if items.is_empty() {
                return Err(
                    Diag::new("empty list is not a valid expression").with_span(span.clone())
                );
            }
            let (op, _) = atom_sym(&items[0])
                .ok_or_else(|| Diag::new("call head must be a symbol").with_span(se_span(&items[0])))?;
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
        Sexp::Brack(_, span) => Err(
            Diag::new("unexpected [..] where expression expected").with_span(span.clone()),
        ),
    }
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
    let params = parse_params(&items[2])?;
    let body = parse_expr(&items[3])?;
    Ok(FnDef {
        name,
        params,
        body,
        span: Span {
            start: name_sp.start,
            end: span.end,
        },
    })
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
        let (items, _span) = match se {
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
            "defstruct" => out.push(Top::Struct(parse_struct(se)?)),
            "defn" => out.push(Top::Func(parse_fn(se)?)),
            _ => {
                return Err(Diag::new(format!("unknown top-level form '{}'", head))
                    .with_span(se_span(&items[0])));
            }
        }
    }
    Ok(out)
}

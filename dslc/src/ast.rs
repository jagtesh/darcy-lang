use crate::diag::{Diag, DslResult, Span};
use crate::lexer::TokKind;
use crate::parser::Sexp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Named(String),
    Vec(Box<Ty>),
    Unknown,
}

impl Ty {
    pub fn rust(&self) -> String {
        match self {
            Ty::Named(s) => s.clone(),
            Ty::Vec(inner) => format!("Vec<{}>", inner.rust()),
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
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnionDef {
    pub name: String,
    pub variants: Vec<VariantDef>,
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
    VecLit {
        elems: Vec<Expr>,
        span: Span,
        ann: Option<Ty>,
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
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s) => s.clone(),
            Expr::Float(_, s) => s.clone(),
            Expr::Var(_, s) => s.clone(),
            Expr::VecLit { span, .. } => span.clone(),
            Expr::Field { span, .. } => span.clone(),
            Expr::Match { span, .. } => span.clone(),
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
    Union(UnionDef),
    Func(FnDef),
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
    if let Some(inner) = s.strip_prefix("Vec<").and_then(|rest| rest.strip_suffix('>')) {
        let inner_ty = parse_type_from_sym(inner);
        return Ty::Vec(Box::new(inner_ty));
    }
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
        variants.push(VariantDef {
            name: vname,
            fields,
            span: Span {
                start: vname_sp.start,
                end: vspan.end,
            },
        });
    }

    Ok(UnionDef {
        name,
        variants,
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
            let (op, op_span) = atom_sym(&items[0])
                .ok_or_else(|| Diag::new("call head must be a symbol").with_span(se_span(&items[0])))?;
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

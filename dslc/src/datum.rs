use crate::diag::Span;
use crate::lexer::TokKind;
use crate::parser::Sexp;

#[derive(Debug, Clone)]
pub enum Datum {
    List(Vec<Datum>, Span),
    Vec(Vec<Datum>, Span),
    Map(Vec<(Datum, Datum)>, Span),
    Set(Vec<Datum>, Span),
    Meta {
        meta: Box<Datum>,
        form: Box<Datum>,
        span: Span,
    },
    Symbol(String, Span),
    SymbolLit(String, Span),
    Str(String, Span),
    Int(i64, Span),
    Float(f64, Span),
    Bool(bool, Span),
    Nil(Span),
}

pub fn datum_span(d: &Datum) -> Span {
    match d {
        Datum::List(_, sp)
        | Datum::Vec(_, sp)
        | Datum::Map(_, sp)
        | Datum::Set(_, sp)
        | Datum::Meta { span: sp, .. }
        | Datum::Symbol(_, sp)
        | Datum::SymbolLit(_, sp)
        | Datum::Str(_, sp)
        | Datum::Int(_, sp)
        | Datum::Float(_, sp)
        | Datum::Bool(_, sp)
        | Datum::Nil(sp) => sp.clone(),
    }
}

pub fn datum_to_sexp(d: &Datum) -> Sexp {
    match d {
        Datum::List(items, sp) => Sexp::List(items.iter().map(datum_to_sexp).collect(), sp.clone()),
        Datum::Vec(items, sp) => Sexp::Brack(items.iter().map(datum_to_sexp).collect(), sp.clone()),
        Datum::Map(entries, sp) => {
            let mut items = Vec::new();
            for (k, v) in entries {
                items.push(datum_to_sexp(k));
                items.push(datum_to_sexp(v));
            }
            Sexp::Brace(items, sp.clone())
        }
        Datum::Set(items, sp) => Sexp::Set(items.iter().map(datum_to_sexp).collect(), sp.clone()),
        Datum::Meta { form, .. } => datum_to_sexp(form),
        Datum::Symbol(s, sp) => Sexp::Atom(TokKind::Sym(s.clone()), sp.clone()),
        Datum::SymbolLit(s, sp) => Sexp::Atom(TokKind::Sym(s.clone()), sp.clone()),
        Datum::Str(s, sp) => Sexp::Atom(TokKind::Str(s.clone()), sp.clone()),
        Datum::Int(v, sp) => Sexp::Atom(TokKind::Int(*v), sp.clone()),
        Datum::Float(v, sp) => Sexp::Atom(TokKind::Float(*v), sp.clone()),
        Datum::Bool(b, sp) => {
            let sym = if *b { "true" } else { "false" };
            Sexp::Atom(TokKind::Sym(sym.to_string()), sp.clone())
        }
        Datum::Nil(sp) => Sexp::Atom(TokKind::Sym("nil".to_string()), sp.clone()),
    }
}

pub fn datums_to_sexps(items: &[Datum]) -> Vec<Sexp> {
    items.iter().map(datum_to_sexp).collect()
}

use std::collections::BTreeMap;

use crate::ast::{Def, Expr, FnDef, Ty};
use crate::diag::Span;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpanKey {
    pub start: usize,
    pub end: usize,
}

impl SpanKey {
    pub fn new(span: &Span) -> Self {
        Self {
            start: span.start.byte,
            end: span.end.byte,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CastHint {
    pub span: Span,
    pub target: Ty,
}

#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub expr: Expr,
    pub ty: Ty,
    pub casts: Vec<CastHint>,
    pub types: BTreeMap<SpanKey, Ty>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    ByVal,
    ByRef,
    ByRefNoAmp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericBound {
    Copy,
    Clone,
    Add,
    Sub,
    Mul,
    Div,
    PartialEq,
    PartialOrd,
    Len,
    IsEmpty,
    Push(Ty),
    FromInt,
}

#[derive(Debug, Clone)]
pub struct TypedFn {
    pub def: FnDef,
    pub param_tys: BTreeMap<String, Ty>,
    pub param_modes: BTreeMap<String, ParamMode>,
    pub generic_bounds: BTreeMap<u32, Vec<GenericBound>>,
    pub body: TypedExpr,
    pub mutated: std::collections::BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct TypedDef {
    pub def: Def,
    pub body: TypedExpr,
}

use std::collections::BTreeMap;

use crate::ast::{Expr, FnDef, Ty};
use crate::diag::Span;

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
}

#[derive(Debug, Clone)]
pub struct TypedFn {
    pub def: FnDef,
    pub param_tys: BTreeMap<String, Ty>,
    pub body: TypedExpr,
}

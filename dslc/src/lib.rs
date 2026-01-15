mod ast;
mod diag;
mod lexer;
mod lower;
mod parser;
mod pipeline;
mod typecheck;
mod typed;

pub use ast::{parse_toplevel, Expr, FnDef, StructDef, Top, Ty};
pub use diag::{render_diag, Diag, DslResult, Loc, Span};
pub use lexer::{lex, Tok, TokKind};
pub use lower::lower_program;
pub use pipeline::{analyze, PipelineOutput};
pub use parser::{Parser, Sexp};
pub use typecheck::{typecheck_fn, typecheck_tops, TypeEnv, TypecheckedProgram};
pub use typed::{CastHint, TypedExpr, TypedFn};


pub fn compile(src: &str) -> DslResult<String> {
    let pipeline = analyze(src)?;
    lower_program(&pipeline)
}

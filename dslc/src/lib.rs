mod ast;
mod diag;
mod lexer;
mod lower;
mod module;
mod parser;
mod pipeline;
mod typecheck;
mod typed;

pub use ast::{parse_toplevel, Expr, FnDef, StructDef, Top, Ty, UseDecl};
pub use diag::{render_diag, Diag, DslResult, Loc, Span};
pub use lexer::{lex, Tok, TokKind};
pub use lower::lower_program;
pub use module::compile_modules;
pub use pipeline::{analyze, PipelineOutput};
pub use parser::{Parser, Sexp};
pub use typecheck::{typecheck_fn, typecheck_tops, FnEnv, FnSig, TypeEnv, TypecheckedProgram};
pub use typed::{CastHint, TypedExpr, TypedFn};


pub fn compile(src: &str) -> DslResult<String> {
    let pipeline = analyze(src)?;
    lower_program(&pipeline)
}

pub fn compile_with_modules(
    root_path: &std::path::Path,
    src: &str,
    lib_paths: &[std::path::PathBuf],
) -> DslResult<String> {
    let tops = compile_modules(root_path, src, lib_paths)?;
    let typechecked = typecheck_tops(&tops)?;
    lower_program(&PipelineOutput { tops, typechecked })
}

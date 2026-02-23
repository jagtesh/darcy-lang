mod ast;
mod datum;
mod diag;
mod lexer;
mod lower;
mod macro_expand;
mod module;
mod parser;
mod pipeline;
mod reader;
mod type_aliases;
mod typecheck;
mod typed;

pub use ast::{parse_toplevel, Expr, FnDef, StructDef, Top, Ty, UseDecl};
pub use datum::{datums_to_sexps, Datum};
pub use diag::{render_diag, render_diag_with_level, Diag, DslResult, Loc, Span};
pub use lexer::{lex, Tok, TokKind};
pub use lower::lower_program;
pub use macro_expand::expand_program;
pub use module::compile_modules;
pub use parser::{Parser, Sexp};
pub use pipeline::{analyze, PipelineOutput};
pub use reader::Reader;
pub use typecheck::{typecheck_fn, typecheck_tops, FnEnv, FnSig, TypeEnv, TypecheckedProgram};
pub use typed::{CastHint, TypedExpr, TypedFn};

pub fn compile(src: &str) -> DslResult<String> {
    let pipeline = analyze(src)?;
    lower_program(&pipeline)
}

pub fn read_expand_toplevel(src: &str) -> DslResult<Vec<Top>> {
    let toks = lex(src)?;
    let mut reader = Reader::new(toks);
    let forms = reader.parse_all()?;
    let expanded = expand_program(&forms)?;
    let sexps = datums_to_sexps(&expanded);
    parse_toplevel(&sexps)
}

#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub rust: String,
    pub warnings: Vec<Diag>,
}

pub fn compile_with_modules(
    root_path: &std::path::Path,
    src: &str,
    lib_paths: &[std::path::PathBuf],
) -> DslResult<CompileOutput> {
    let tops = compile_modules(root_path, src, lib_paths)?;
    let typechecked = typecheck_tops(&tops)?;
    let rust = lower_program(&PipelineOutput {
        tops,
        typechecked: typechecked.clone(),
    })?;
    Ok(CompileOutput {
        rust,
        warnings: typechecked.warnings,
    })
}

mod ast;
mod datum;
mod diag;
mod feedback;
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
pub use feedback::{apply_feedback_hints, collect_feedback_hints, FeedbackHints};
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

#[derive(Debug, Clone, Copy)]
pub struct CompileOptions {
    pub feedback: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self { feedback: false }
    }
}

pub fn compile_with_modules(
    root_path: &std::path::Path,
    src: &str,
    lib_paths: &[std::path::PathBuf],
) -> DslResult<CompileOutput> {
    compile_with_modules_opts(root_path, src, lib_paths, CompileOptions::default())
}

pub fn compile_with_modules_opts(
    root_path: &std::path::Path,
    src: &str,
    lib_paths: &[std::path::PathBuf],
    opts: CompileOptions,
) -> DslResult<CompileOutput> {
    let tops = compile_modules(root_path, src, lib_paths)?;
    let mut typechecked = typecheck_tops(&tops)?;
    let rust_first = lower_program(&PipelineOutput {
        tops: tops.clone(),
        typechecked: typechecked.clone(),
    })?;
    if opts.feedback {
        let hints = collect_feedback_hints(&rust_first)?;
        apply_feedback_hints(&mut typechecked, &hints);
    }
    let rust = lower_program(&PipelineOutput {
        tops,
        typechecked: typechecked.clone(),
    })?;
    Ok(CompileOutput {
        rust,
        warnings: typechecked.warnings,
    })
}

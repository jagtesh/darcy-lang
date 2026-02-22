use crate::ast::Top;
use crate::datum::datums_to_sexps;
use crate::diag::DslResult;
use crate::lexer::lex;
use crate::macro_expand::expand_program;
use crate::parse_toplevel;
use crate::reader::Reader;
use crate::typecheck::{expand_inline_tops, typecheck_tops, TypecheckedProgram};

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub tops: Vec<Top>,
    pub typechecked: TypecheckedProgram,
}

pub fn analyze(src: &str) -> DslResult<PipelineOutput> {
    let toks = lex(src)?;
    let mut reader = Reader::new(toks);
    let forms = reader.parse_all()?;
    let expanded = expand_program(&forms)?;
    let sexps = datums_to_sexps(&expanded);
    let tops = parse_toplevel(&sexps)?;
    let tops = expand_inline_tops(&tops)?;
    let typechecked = typecheck_tops(&tops)?;
    Ok(PipelineOutput { tops, typechecked })
}

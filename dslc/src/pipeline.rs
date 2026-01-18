use crate::ast::Top;
use crate::diag::DslResult;
use crate::lexer::lex;
use crate::parser::Parser;
use crate::typecheck::{expand_inline_tops, typecheck_tops, TypecheckedProgram};
use crate::parse_toplevel;

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub tops: Vec<Top>,
    pub typechecked: TypecheckedProgram,
}

pub fn analyze(src: &str) -> DslResult<PipelineOutput> {
    let toks = lex(src)?;
    let mut p = Parser::new(toks);
    let sexps = p.parse_all()?;
    let tops = parse_toplevel(&sexps)?;
    let tops = expand_inline_tops(&tops)?;
    let typechecked = typecheck_tops(&tops)?;
    Ok(PipelineOutput { tops, typechecked })
}

use crate::diag::{Diag, DslResult};
use crate::lexer::{Tok, TokKind};
use crate::diag::Span;

#[derive(Debug, Clone)]
pub enum Sexp {
    Atom(TokKind, Span),
    List(Vec<Sexp>, Span),
    Brack(Vec<Sexp>, Span),
}

pub struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    pub fn new(toks: Vec<Tok>) -> Self {
        Self { toks, pos: 0 }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, k: TokKind) -> DslResult<Tok> {
        let t = self
            .bump()
            .ok_or_else(|| Diag::new("unexpected end of input"))?;
        if std::mem::discriminant(&t.kind) == std::mem::discriminant(&k) {
            Ok(t)
        } else {
            Err(Diag::new(format!("expected {:?}, got {:?}", k, t.kind)).with_span(t.span))
        }
    }

    pub fn parse_all(&mut self) -> DslResult<Vec<Sexp>> {
        let mut out = Vec::new();
        while self.peek().is_some() {
            out.push(self.parse_one()?);
        }
        Ok(out)
    }

    fn parse_one(&mut self) -> DslResult<Sexp> {
        let t = self
            .peek()
            .ok_or_else(|| Diag::new("unexpected end of input"))?
            .clone();
        match &t.kind {
            TokKind::LParen => self.parse_list(),
            TokKind::LBrack => self.parse_brack(),
            TokKind::RParen | TokKind::RBrack => {
                Err(Diag::new("unexpected closing delimiter").with_span(t.span))
            }
            _ => {
                let t = self.bump().unwrap();
                Ok(Sexp::Atom(t.kind, t.span))
            }
        }
    }

    fn parse_list(&mut self) -> DslResult<Sexp> {
        let open = self.expect(TokKind::LParen)?;
        let mut items = Vec::new();
        while let Some(t) = self.peek() {
            match t.kind {
                TokKind::RParen => {
                    let close = self.bump().unwrap();
                    let span = Span {
                        start: open.span.start,
                        end: close.span.end,
                    };
                    return Ok(Sexp::List(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '('").with_span(open.span))
    }

    fn parse_brack(&mut self) -> DslResult<Sexp> {
        let open = self.expect(TokKind::LBrack)?;
        let mut items = Vec::new();
        while let Some(t) = self.peek() {
            match t.kind {
                TokKind::RBrack => {
                    let close = self.bump().unwrap();
                    let span = Span {
                        start: open.span.start,
                        end: close.span.end,
                    };
                    return Ok(Sexp::Brack(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '['").with_span(open.span))
    }
}

use crate::diag::Span;
use crate::diag::{Diag, DslResult};
use crate::lexer::{Tok, TokKind};

#[derive(Debug, Clone)]
pub enum Sexp {
    Atom(TokKind, Span),
    List(Vec<Sexp>, Span),
    Brack(Vec<Sexp>, Span),
    Brace(Vec<Sexp>, Span),
    Set(Vec<Sexp>, Span),
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
        loop {
            let t = self
                .peek()
                .ok_or_else(|| Diag::new("unexpected end of input"))?
                .clone();
            match &t.kind {
                TokKind::Discard => {
                    let discard = self.bump().unwrap();
                    let _ = self.parse_one().map_err(|_| {
                        Diag::new("discard must be followed by a form").with_span(discard.span)
                    })?;
                    continue;
                }
                TokKind::Quote
                | TokKind::SyntaxQuote
                | TokKind::Unquote
                | TokKind::UnquoteSplicing => {
                    let quote = self.bump().unwrap();
                    let inner = self.parse_one()?;
                    let head = match quote.kind {
                        TokKind::Quote => "quote",
                        TokKind::SyntaxQuote => "syntax-quote",
                        TokKind::Unquote => "unquote",
                        TokKind::UnquoteSplicing => "unquote-splicing",
                        _ => "quote",
                    };
                    let span = Span {
                        start: quote.span.start,
                        end: sexp_span(&inner).end,
                    };
                    let quote_atom = Sexp::Atom(TokKind::Sym(head.to_string()), quote.span);
                    return Ok(Sexp::List(vec![quote_atom, inner], span));
                }
                TokKind::Meta => {
                    let meta = self.bump().unwrap();
                    let meta_form = self.parse_one()?;
                    let form = self.parse_one()?;
                    let span = Span {
                        start: meta.span.start,
                        end: sexp_span(&form).end,
                    };
                    let head = Sexp::Atom(TokKind::Sym("with-meta".to_string()), meta.span);
                    return Ok(Sexp::List(vec![head, form, meta_form], span));
                }
                TokKind::LParen => return self.parse_list(),
                TokKind::LBrack => return self.parse_brack(),
                TokKind::LBrace => return self.parse_brace(),
                TokKind::LSet => return self.parse_set(),
                TokKind::RParen | TokKind::RBrack | TokKind::RBrace => {
                    return Err(Diag::new("unexpected closing delimiter").with_span(t.span))
                }
                _ => {
                    let t = self.bump().unwrap();
                    return Ok(Sexp::Atom(t.kind, t.span));
                }
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

    fn parse_brace(&mut self) -> DslResult<Sexp> {
        let open = self.expect(TokKind::LBrace)?;
        let mut items = Vec::new();
        while let Some(t) = self.peek() {
            match t.kind {
                TokKind::RBrace => {
                    let close = self.bump().unwrap();
                    let span = Span {
                        start: open.span.start,
                        end: close.span.end,
                    };
                    return Ok(Sexp::Brace(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '{'").with_span(open.span))
    }

    fn parse_set(&mut self) -> DslResult<Sexp> {
        let open = self.expect(TokKind::LSet)?;
        let mut items = Vec::new();
        while let Some(t) = self.peek() {
            match t.kind {
                TokKind::RBrace => {
                    let close = self.bump().unwrap();
                    let span = Span {
                        start: open.span.start,
                        end: close.span.end,
                    };
                    return Ok(Sexp::Set(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '#{'").with_span(open.span))
    }
}

fn sexp_span(se: &Sexp) -> Span {
    match se {
        Sexp::Atom(_, sp) => sp.clone(),
        Sexp::List(_, sp) => sp.clone(),
        Sexp::Brack(_, sp) => sp.clone(),
        Sexp::Brace(_, sp) => sp.clone(),
        Sexp::Set(_, sp) => sp.clone(),
    }
}

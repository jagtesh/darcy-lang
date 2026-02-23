use crate::datum::Datum;
use crate::diag::{Diag, DslResult, Span};
use crate::lexer::{Tok, TokKind};

pub struct Reader {
    toks: Vec<Tok>,
    pos: usize,
}

impl Reader {
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

    pub fn parse_all(&mut self) -> DslResult<Vec<Datum>> {
        let mut out = Vec::new();
        while self.peek().is_some() {
            out.push(self.parse_one()?);
        }
        Ok(out)
    }

    fn parse_one(&mut self) -> DslResult<Datum> {
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
                        end: datum_span(&inner).end,
                    };
                    let quote_sym = Datum::Symbol(head.to_string(), quote.span);
                    return Ok(Datum::List(vec![quote_sym, inner], span));
                }
                TokKind::Meta => {
                    let meta = self.bump().unwrap();
                    let meta_form = self.parse_one()?;
                    let form = self.parse_one()?;
                    let span = Span {
                        start: meta.span.start,
                        end: datum_span(&form).end,
                    };
                    return Ok(Datum::Meta {
                        meta: Box::new(meta_form),
                        form: Box::new(form),
                        span,
                    });
                }
                TokKind::LParen => return self.parse_list(),
                TokKind::LBrack => return self.parse_vec(),
                TokKind::LBrace => return self.parse_map(),
                TokKind::LSet => return self.parse_set(),
                TokKind::RParen | TokKind::RBrack | TokKind::RBrace => {
                    return Err(Diag::new("unexpected closing delimiter").with_span(t.span))
                }
                TokKind::Sym(s) => {
                    let t = self.bump().unwrap();
                    if s == "true" {
                        return Ok(Datum::Bool(true, t.span));
                    }
                    if s == "false" {
                        return Ok(Datum::Bool(false, t.span));
                    }
                    if s == "nil" {
                        return Ok(Datum::Nil(t.span));
                    }
                    if s.starts_with(':') {
                        return Ok(Datum::SymbolLit(s.clone(), t.span));
                    }
                    return Ok(Datum::Symbol(s.clone(), t.span));
                }
                TokKind::Str(s) => {
                    let t = self.bump().unwrap();
                    return Ok(Datum::Str(s.clone(), t.span));
                }
                TokKind::Int(v) => {
                    let t = self.bump().unwrap();
                    return Ok(Datum::Int(*v, t.span));
                }
                TokKind::Float(v) => {
                    let t = self.bump().unwrap();
                    return Ok(Datum::Float(*v, t.span));
                }
            }
        }
    }

    fn parse_list(&mut self) -> DslResult<Datum> {
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
                    return Ok(Datum::List(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '('").with_span(open.span))
    }

    fn parse_vec(&mut self) -> DslResult<Datum> {
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
                    return Ok(Datum::Vec(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '['").with_span(open.span))
    }

    fn parse_map(&mut self) -> DslResult<Datum> {
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
                    if items.len() % 2 != 0 {
                        return Err(
                            Diag::new("map literal requires even number of forms").with_span(span)
                        );
                    }
                    let mut entries = Vec::new();
                    let mut iter = items.into_iter();
                    while let Some(k) = iter.next() {
                        let v = iter.next().expect("even number of map entries");
                        entries.push((k, v));
                    }
                    return Ok(Datum::Map(entries, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '{'").with_span(open.span))
    }

    fn parse_set(&mut self) -> DslResult<Datum> {
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
                    return Ok(Datum::Set(items, span));
                }
                _ => items.push(self.parse_one()?),
            }
        }
        Err(Diag::new("unclosed '#{'").with_span(open.span))
    }
}

fn datum_span(d: &Datum) -> Span {
    match d {
        Datum::List(_, sp)
        | Datum::Vec(_, sp)
        | Datum::Map(_, sp)
        | Datum::Set(_, sp)
        | Datum::Meta { span: sp, .. }
        | Datum::Symbol(_, sp)
        | Datum::SymbolLit(_, sp)
        | Datum::Str(_, sp)
        | Datum::Int(_, sp)
        | Datum::Float(_, sp)
        | Datum::Bool(_, sp)
        | Datum::Nil(sp) => sp.clone(),
    }
}

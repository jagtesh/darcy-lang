use crate::diag::{Diag, DslResult, Loc, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokKind {
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    LSet,
    Quote,
    SyntaxQuote,
    Unquote,
    UnquoteSplicing,
    Meta,
    Discard,
    Sym(String),
    Str(String),
    Int(i64),
    Float(f64),
}

#[derive(Debug, Clone)]
pub struct Tok {
    pub kind: TokKind,
    pub span: Span,
}

fn is_sym_start(c: char) -> bool {
    c.is_ascii_alphabetic() || "_:+-*/<>=!?&|.".contains(c)
}

fn is_sym_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_+-*/<>=!?&|.".contains(c) || c == ':' || c == '/' || c == '#'
}

pub fn lex(input: &str) -> DslResult<Vec<Tok>> {
    let mut toks = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    let bytes = input.as_bytes();
    while i < bytes.len() {
        let c = input[i..].chars().next().unwrap();
        let start = Loc { line, col, byte: i };

        if c == ';' {
            while i < bytes.len() {
                let ch = input[i..].chars().next().unwrap();
                if ch == '\n' {
                    break;
                }
                i += ch.len_utf8();
                col += 1;
            }
            continue;
        }
        if c == '#' && i + 1 < bytes.len() && input[i + 1..].chars().next() == Some('_') {
            i += 2;
            col += 2;
            toks.push(Tok {
                kind: TokKind::Discard,
                span: Span {
                    start,
                    end: Loc { line, col, byte: i },
                },
            });
            continue;
        }
        if c == '#' && i + 1 < bytes.len() && input[i + 1..].chars().next() == Some('|') {
            i += 2;
            col += 2;
            let mut closed = false;
            while i < bytes.len() {
                let ch = input[i..].chars().next().unwrap();
                if ch == '|' && i + 1 < bytes.len() && input[i + 1..].chars().next() == Some('#') {
                    i += 2;
                    col += 2;
                    closed = true;
                    break;
                }
                if ch == '\n' {
                    i += 1;
                    line += 1;
                    col = 1;
                } else {
                    i += ch.len_utf8();
                    col += 1;
                }
            }
            if !closed {
                return Err(Diag::new("unterminated block comment").with_span(Span {
                    start,
                    end: Loc { line, col, byte: i },
                }));
            }
            continue;
        }
        if c == '#' && i + 1 < bytes.len() && input[i + 1..].chars().next() == Some('{') {
            i += 2;
            col += 2;
            toks.push(Tok {
                kind: TokKind::LSet,
                span: Span {
                    start,
                    end: Loc { line, col, byte: i },
                },
            });
            continue;
        }
        if c == ',' {
            i += 1;
            col += 1;
            continue;
        }
        if c.is_whitespace() {
            if c == '\n' {
                i += 1;
                line += 1;
                col = 1;
            } else {
                i += c.len_utf8();
                col += 1;
            }
            continue;
        }

        let kind = match c {
            '(' => {
                i += 1;
                col += 1;
                TokKind::LParen
            }
            ')' => {
                i += 1;
                col += 1;
                TokKind::RParen
            }
            '[' => {
                i += 1;
                col += 1;
                TokKind::LBrack
            }
            ']' => {
                i += 1;
                col += 1;
                TokKind::RBrack
            }
            '{' => {
                i += 1;
                col += 1;
                TokKind::LBrace
            }
            '}' => {
                i += 1;
                col += 1;
                TokKind::RBrace
            }
            '\'' => {
                i += 1;
                col += 1;
                TokKind::Quote
            }
            '`' => {
                i += 1;
                col += 1;
                TokKind::SyntaxQuote
            }
            '~' => {
                if i + 1 < bytes.len() && input[i + 1..].chars().next() == Some('@') {
                    i += 2;
                    col += 2;
                    TokKind::UnquoteSplicing
                } else {
                    i += 1;
                    col += 1;
                    TokKind::Unquote
                }
            }
            '^' => {
                i += 1;
                col += 1;
                TokKind::Meta
            }
            '"' => {
                i += 1;
                col += 1;
                let mut s = String::new();
                let mut closed = false;
                while i < bytes.len() {
                    let ch = input[i..].chars().next().unwrap();
                    if ch == '"' {
                        i += 1;
                        col += 1;
                        closed = true;
                        break;
                    }
                    if ch == '\\' {
                        if i + 1 >= bytes.len() {
                            return Err(Diag::new("unterminated string escape").with_span(Span {
                                start,
                                end: Loc { line, col, byte: i },
                            }));
                        }
                        let next = input[i + 1..].chars().next().unwrap();
                        match next {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            '\\' => s.push('\\'),
                            '"' => s.push('"'),
                            _ => {
                                return Err(Diag::new("unknown string escape").with_span(Span {
                                    start,
                                    end: Loc {
                                        line,
                                        col,
                                        byte: i + 2,
                                    },
                                }))
                            }
                        }
                        i += 2;
                        col += 2;
                        continue;
                    }
                    if ch == '\n' {
                        return Err(Diag::new("unterminated string literal").with_span(Span {
                            start,
                            end: Loc { line, col, byte: i },
                        }));
                    }
                    s.push(ch);
                    i += ch.len_utf8();
                    col += 1;
                }
                if !closed {
                    return Err(Diag::new("unterminated string literal").with_span(Span {
                        start,
                        end: Loc { line, col, byte: i },
                    }));
                }
                TokKind::Str(s)
            }
            _ => {
                if c.is_ascii_digit()
                    || (c == '-'
                        && input[i + 1..]
                            .chars()
                            .next()
                            .map(|x| x.is_ascii_digit())
                            .unwrap_or(false))
                {
                    let mut j = i;
                    let mut saw_dot = false;
                    let mut saw_exp = false;
                    let mut first = true;
                    while j < bytes.len() {
                        let ch = input[j..].chars().next().unwrap();
                        if first && ch == '-' {
                        } else if ch.is_ascii_digit() {
                        } else if ch == '.' && !saw_dot && !saw_exp {
                            saw_dot = true;
                        } else if (ch == 'e' || ch == 'E') && !saw_exp {
                            saw_exp = true;
                        } else if saw_exp && (ch == '+' || ch == '-') {
                        } else {
                            break;
                        }
                        first = false;
                        j += ch.len_utf8();
                    }
                    let s = &input[i..j];
                    i = j;
                    col += s.chars().count();
                    if saw_dot || saw_exp {
                        let v: f64 = s.parse().map_err(|_| {
                            Diag::new(format!("invalid float literal: {}", s)).with_span(Span {
                                start,
                                end: Loc { line, col, byte: i },
                            })
                        })?;
                        TokKind::Float(v)
                    } else {
                        let v: i64 = s.parse().map_err(|_| {
                            Diag::new(format!("invalid int literal: {}", s)).with_span(Span {
                                start,
                                end: Loc { line, col, byte: i },
                            })
                        })?;
                        TokKind::Int(v)
                    }
                } else if is_sym_start(c) {
                    let mut j = i;
                    let mut type_depth = 0u32;
                    while j < bytes.len() {
                        let ch = input[j..].chars().next().unwrap();
                        if ch == '<' {
                            type_depth += 1;
                            j += ch.len_utf8();
                            continue;
                        }
                        if ch == '>' {
                            if type_depth > 0 {
                                type_depth -= 1;
                            }
                            j += ch.len_utf8();
                            continue;
                        }
                        if ch == ',' && type_depth > 0 {
                            j += ch.len_utf8();
                            continue;
                        }
                        if is_sym_char(ch) {
                            j += ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let s = &input[i..j];
                    i = j;
                    col += s.chars().count();
                    TokKind::Sym(s.to_string())
                } else {
                    return Err(
                        Diag::new(format!("unexpected character: '{}'", c)).with_span(Span {
                            start,
                            end: Loc {
                                line,
                                col,
                                byte: i + c.len_utf8(),
                            },
                        }),
                    );
                }
            }
        };

        let end = Loc { line, col, byte: i };
        toks.push(Tok {
            kind,
            span: Span { start, end },
        });
    }

    Ok(toks)
}

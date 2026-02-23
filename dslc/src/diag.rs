#[derive(Debug, Clone, Copy)]
pub struct Loc {
    pub line: usize,
    pub col: usize,
    pub byte: usize,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub start: Loc,
    pub end: Loc,
}

#[derive(Debug, Clone)]
pub struct Diag {
    pub message: String,
    pub span: Option<Span>,
}

impl Diag {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            span: None,
        }
    }
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

pub type DslResult<T> = Result<T, Diag>;

pub fn render_diag(file: &str, src: &str, d: &Diag) -> String {
    render_diag_with_level(file, src, d, "error")
}

pub fn render_diag_with_level(file: &str, src: &str, d: &Diag, level: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("{}: {}\n", level, d.message));
    if let Some(sp) = &d.span {
        s.push_str(&format!(
            " --> {}:{}:{}\n",
            file, sp.start.line, sp.start.col
        ));
        if let Some(line) = src.lines().nth(sp.start.line.saturating_sub(1)) {
            let width = sp.start.line.to_string().len();
            let pad = " ".repeat(width);
            s.push_str(&format!(
                " {} |\n {:>width$} | {}\n {} |",
                pad,
                sp.start.line,
                line,
                pad,
                width = width
            ));
            let caret_pos = sp.start.col.saturating_sub(1);
            s.push_str(&format!("\n {} | {}^\n", pad, " ".repeat(caret_pos)));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diag_gutter_pipe_stays_aligned_single_digit() {
        let src = "(def PI 3.14)\n";
        let d = Diag::new("boom").with_span(Span {
            start: Loc {
                line: 1,
                col: 6,
                byte: 5,
            },
            end: Loc {
                line: 1,
                col: 8,
                byte: 7,
            },
        });
        let out = render_diag("x.dsl", src, &d);
        let lines: Vec<&str> = out.lines().collect();
        let i_blank = lines
            .iter()
            .position(|l| l.contains("|") && l.trim() == "|")
            .unwrap();
        let i_code = i_blank + 1;
        let i_caret = i_blank + 2;
        let p_blank = lines[i_blank].find('|').unwrap();
        let p_code = lines[i_code].find('|').unwrap();
        let p_caret = lines[i_caret].find('|').unwrap();
        assert_eq!(p_blank, p_code);
        assert_eq!(p_code, p_caret);
    }

    #[test]
    fn diag_gutter_pipe_stays_aligned_multi_digit() {
        let mut src = String::new();
        for _ in 0..12 {
            src.push_str("(noop)\n");
        }
        let d = Diag::new("boom").with_span(Span {
            start: Loc {
                line: 12,
                col: 2,
                byte: 0,
            },
            end: Loc {
                line: 12,
                col: 3,
                byte: 0,
            },
        });
        let out = render_diag("x.dsl", &src, &d);
        let lines: Vec<&str> = out.lines().collect();
        let i_blank = lines
            .iter()
            .position(|l| l.contains("|") && l.trim() == "|")
            .unwrap();
        let i_code = i_blank + 1;
        let i_caret = i_blank + 2;
        let p_blank = lines[i_blank].find('|').unwrap();
        let p_code = lines[i_code].find('|').unwrap();
        let p_caret = lines[i_caret].find('|').unwrap();
        assert_eq!(p_blank, p_code);
        assert_eq!(p_code, p_caret);
    }
}

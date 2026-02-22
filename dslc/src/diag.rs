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
            s.push_str(&format!("  |\n{:>2} | {}\n  |", sp.start.line, line));
            let caret_pos = sp.start.col.saturating_sub(1);
            s.push_str(&format!("\n  | {}^\n", " ".repeat(caret_pos)));
        }
    }
    s
}

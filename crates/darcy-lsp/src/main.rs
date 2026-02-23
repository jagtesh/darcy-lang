use std::collections::BTreeMap;
use std::sync::Arc;

use dslc::{lex, Tok, TokKind};
use dslc::{Diag, Span};
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod defs;

#[derive(Clone)]
struct SharedDefs {
    keywords: Vec<String>,
    types: Vec<String>,
    builtins: Vec<String>,
}

impl SharedDefs {
    fn new() -> Self {
        let defs = defs::Defs::load();
        Self {
            keywords: defs.keywords,
            types: defs.types,
            builtins: defs.builtins,
        }
    }
}

#[derive(Default)]
struct DocState {
    text: String,
}

struct Backend {
    client: Client,
    docs: Arc<Mutex<BTreeMap<Url, DocState>>>,
    defs: SharedDefs,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            docs: Arc::new(Mutex::new(BTreeMap::new())),
            defs: SharedDefs::new(),
        }
    }

    async fn update_doc(&self, uri: Url, text: String) {
        let mut docs = self.docs.lock().await;
        docs.insert(uri, DocState { text });
    }

    async fn get_doc(&self, uri: &Url) -> Option<String> {
        let docs = self.docs.lock().await;
        docs.get(uri).map(|d| d.text.clone())
    }

    async fn publish_diagnostics(&self, uri: &Url, text: &str) {
        let diags = match analyze(text) {
            Ok(_) => Vec::new(),
            Err(d) => vec![diag_to_lsp(&d)],
        };
        self.client
            .publish_diagnostics(uri.clone(), diags, None)
            .await;
    }

    fn completion_items(&self, tops: &[dslc::Top]) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        for kw in &self.defs.keywords {
            items.push(CompletionItem {
                label: kw.clone(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..CompletionItem::default()
            });
        }
        for ty in &self.defs.types {
            items.push(CompletionItem {
                label: ty.clone(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                ..CompletionItem::default()
            });
        }
        for b in &self.defs.builtins {
            items.push(CompletionItem {
                label: b.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                ..CompletionItem::default()
            });
        }
        for t in tops {
            match t {
                dslc::Top::Struct(s) => items.push(CompletionItem {
                    label: s.name.clone(),
                    kind: Some(CompletionItemKind::STRUCT),
                    ..CompletionItem::default()
                }),
                dslc::Top::Union(u) => items.push(CompletionItem {
                    label: u.name.clone(),
                    kind: Some(CompletionItemKind::ENUM),
                    ..CompletionItem::default()
                }),
                dslc::Top::Func(f) => items.push(CompletionItem {
                    label: f.name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    ..CompletionItem::default()
                }),
                dslc::Top::Def(d) => items.push(CompletionItem {
                    label: d.name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    ..CompletionItem::default()
                }),
                dslc::Top::Inline(i) => items.push(CompletionItem {
                    label: i.name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    ..CompletionItem::default()
                }),
                _ => {}
            }
        }
        items
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        let caps = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec!["/".to_string(), "-".to_string(), ":".to_string()]),
                ..CompletionOptions::default()
            }),
            definition_provider: Some(OneOf::Left(true)),
            document_formatting_provider: Some(OneOf::Left(true)),
            ..ServerCapabilities::default()
        };
        Ok(InitializeResult {
            capabilities: caps,
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Darcy LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.update_doc(uri.clone(), text.clone()).await;
        self.publish_diagnostics(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params
            .content_changes
            .last()
            .map(|c| c.text.clone())
            .unwrap_or_default();
        self.update_doc(uri.clone(), text.clone()).await;
        self.publish_diagnostics(&uri, &text).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.update_doc(params.text_document.uri.clone(), text.clone())
                .await;
            self.publish_diagnostics(&params.text_document.uri, &text)
                .await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let text = match self.get_doc(&uri).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let analysis = match analyze(&text) {
            Ok(a) => a,
            Err(_) => return Ok(None),
        };
        let offset = match pos_to_byte(&text, pos) {
            Some(o) => o,
            None => return Ok(None),
        };
        if let Some(ty) = find_type_at_offset(&analysis, offset) {
            let contents = HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("`{}`", ty.rust()),
            });
            return Ok(Some(Hover {
                contents,
                range: None,
            }));
        }
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let text = match self.get_doc(&uri).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let analysis = match analyze(&text) {
            Ok(a) => a,
            Err(_) => return Ok(None),
        };
        let items = self.completion_items(&analysis.tops);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let text = match self.get_doc(&uri).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let analysis = match analyze(&text) {
            Ok(a) => a,
            Err(_) => return Ok(None),
        };
        let token = match token_at_position(&analysis.tokens, pos) {
            Some(t) => t,
            None => return Ok(None),
        };
        let name = match &token.kind {
            TokKind::Sym(s) => base_symbol(s),
            _ => return Ok(None),
        };
        if let Some(span) = find_definition(&analysis.tops, &name) {
            let range = span_to_range(&span);
            let loc = Location { uri, range };
            return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
        }
        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let text = match self.get_doc(&uri).await {
            Some(t) => t,
            None => return Ok(None),
        };
        let formatted = format_simple(&text);
        if formatted == text {
            return Ok(None);
        }
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(u32::MAX, 0),
        };
        Ok(Some(vec![TextEdit {
            range,
            new_text: formatted,
        }]))
    }
}

struct Analysis {
    tops: Vec<dslc::Top>,
    typechecked: dslc::TypecheckedProgram,
    tokens: Vec<Tok>,
}

fn analyze(text: &str) -> std::result::Result<Analysis, Diag> {
    let tokens = lex(text)?;
    let pipeline = dslc::analyze(text)?;
    Ok(Analysis {
        tops: pipeline.tops,
        typechecked: pipeline.typechecked,
        tokens,
    })
}

fn diag_to_lsp(d: &Diag) -> Diagnostic {
    let range = d.span.as_ref().map(span_to_range).unwrap_or_else(|| Range {
        start: Position::new(0, 0),
        end: Position::new(0, 0),
    });
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        message: d.message.clone(),
        ..Diagnostic::default()
    }
}

fn span_to_range(span: &Span) -> Range {
    Range {
        start: Position::new(
            (span.start.line.saturating_sub(1)) as u32,
            (span.start.col.saturating_sub(1)) as u32,
        ),
        end: Position::new(
            (span.end.line.saturating_sub(1)) as u32,
            (span.end.col.saturating_sub(1)) as u32,
        ),
    }
}

fn token_at_position(tokens: &[Tok], pos: Position) -> Option<Tok> {
    let line = (pos.line + 1) as usize;
    let col = (pos.character + 1) as usize;
    for tok in tokens {
        if span_contains(&tok.span, line, col) {
            return Some(tok.clone());
        }
    }
    None
}

fn span_contains(span: &Span, line: usize, col: usize) -> bool {
    if line < span.start.line || line > span.end.line {
        return false;
    }
    if line == span.start.line && col < span.start.col {
        return false;
    }
    if line == span.end.line && col >= span.end.col {
        return false;
    }
    true
}

fn pos_to_byte(text: &str, pos: Position) -> Option<usize> {
    let target_line = pos.line as usize;
    let target_col = pos.character as usize;
    let mut line = 0usize;
    let mut col = 0usize;
    for (idx, ch) in text.char_indices() {
        if line == target_line && col == target_col {
            return Some(idx);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    if line == target_line && col == target_col {
        return Some(text.len());
    }
    None
}

fn find_type_at_offset(analysis: &Analysis, offset: usize) -> Option<dslc::Ty> {
    let mut best: Option<(usize, dslc::Ty)> = None;
    for typed in &analysis.typechecked.typed_fns {
        for (span_key, ty) in &typed.body.types {
            if span_key.start <= offset && offset < span_key.end {
                let size = span_key.end.saturating_sub(span_key.start);
                if best.as_ref().map_or(true, |(s, _)| size < *s) {
                    best = Some((size, ty.clone()));
                }
            }
        }
    }
    for typed in &analysis.typechecked.typed_defs {
        for (span_key, ty) in &typed.body.types {
            if span_key.start <= offset && offset < span_key.end {
                let size = span_key.end.saturating_sub(span_key.start);
                if best.as_ref().map_or(true, |(s, _)| size < *s) {
                    best = Some((size, ty.clone()));
                }
            }
        }
    }
    best.map(|(_, ty)| ty)
}

fn base_symbol(sym: &str) -> String {
    let sym = sym.rsplit('/').next().unwrap_or(sym);
    sym.to_string()
}

fn find_definition(tops: &[dslc::Top], name: &str) -> Option<Span> {
    for t in tops {
        match t {
            dslc::Top::Struct(s) if s.name == name => return Some(s.span.clone()),
            dslc::Top::Union(u) if u.name == name => return Some(u.span.clone()),
            dslc::Top::Func(f) if f.name == name => return Some(f.span.clone()),
            dslc::Top::Def(d) if d.name == name => return Some(d.span.clone()),
            dslc::Top::Inline(i) if i.name == name => return Some(i.span.clone()),
            _ => {}
        }
    }
    None
}

fn format_simple(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

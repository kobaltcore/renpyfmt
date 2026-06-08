use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{stdin, stdout};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::index::{IndexedReference, IndexedSymbol, IndexedSymbolKind, SymbolLanguage};
use crate::formatter::PythonFormatConfig;
use crate::parse::{ParseOptions, ParseSeverity, parse_document};
use crate::project::format_source;
use crate::source::SourceDocument;
use crate::workspace::WorkspaceState;

pub struct Backend {
    client: Client,
    workspace: Arc<RwLock<WorkspaceState>>,
    python_format_config: PythonFormatConfig,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspace: Arc::new(RwLock::new(WorkspaceState::default())),
            python_format_config: PythonFormatConfig::default(),
        }
    }

    async fn refresh_document(&self, uri: Url, text: String) {
        let Some(path) = file_path_from_uri(&uri) else {
            return;
        };
        let document = parse_document(SourceDocument::new(Some(uri.clone()), path, text), ParseOptions);
        let diagnostics = document
            .diagnostics
            .iter()
            .map(|diagnostic| Diagnostic {
                range: diagnostic.range,
                severity: Some(match diagnostic.severity {
                    ParseSeverity::Error => DiagnosticSeverity::ERROR,
                    ParseSeverity::Warning => DiagnosticSeverity::WARNING,
                    ParseSeverity::Information => DiagnosticSeverity::INFORMATION,
                }),
                source: Some(diagnostic.source.to_string()),
                message: diagnostic.message.clone(),
                ..Diagnostic::default()
            })
            .collect::<Vec<_>>();

        self.workspace.write().await.upsert(document);
        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }

    async fn document_text(&self, uri: &Url) -> Option<(PathBuf, String)> {
        let path = file_path_from_uri(uri)?;
        let document = self.workspace.read().await.document(uri)?;
        Some((path, document.source.text.clone()))
    }
}

const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::CLASS,
    SemanticTokenType::NAMESPACE,
];

fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
        token_modifiers: Vec::new(),
    }
}

#[derive(Clone, Copy)]
struct TokenSpan {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
}

fn compute_semantic_tokens(document: &crate::parse::ParsedDocument) -> SemanticTokens {
    let mut spans = Vec::new();
    collect_keyword_tokens(document, &mut spans);
    collect_symbol_tokens(document, &mut spans);
    collect_reference_tokens(document, &mut spans);
    spans.sort_by_key(|span| (span.line, span.start, span.length, span.token_type));
    spans.dedup_by_key(|span| (span.line, span.start, span.length, span.token_type));

    let mut data = Vec::with_capacity(spans.len());
    let mut previous_line = 0u32;
    let mut previous_start = 0u32;

    for (index, span) in spans.into_iter().enumerate() {
        let delta_line = if index == 0 {
            span.line
        } else {
            span.line.saturating_sub(previous_line)
        };
        let delta_start = if index == 0 || delta_line > 0 {
            span.start
        } else {
            span.start.saturating_sub(previous_start)
        };
        data.push(SemanticToken {
            delta_line,
            delta_start,
            length: span.length,
            token_type: span.token_type,
            token_modifiers_bitset: 0,
        });
        previous_line = span.line;
        previous_start = span.start;
    }

    SemanticTokens {
        result_id: None,
        data,
    }
}

fn collect_symbol_tokens(document: &crate::parse::ParsedDocument, spans: &mut Vec<TokenSpan>) {
    let Some(uri) = document.source.file_url() else {
        return;
    };
    for symbol in &document.symbols.symbols {
        if symbol.definition.uri != uri {
            continue;
        }
        if let Some(span) = span_from_range(&symbol.definition.range, token_type_for_symbol(symbol)) {
            spans.push(span);
        }
    }
}

fn collect_reference_tokens(document: &crate::parse::ParsedDocument, spans: &mut Vec<TokenSpan>) {
    let Some(uri) = document.source.file_url() else {
        return;
    };
    for reference in &document.symbols.references {
        if reference.range.uri != uri {
            continue;
        }
        if let Some(span) = span_from_range(&reference.range.range, token_type_for_reference(reference)) {
            spans.push(span);
        }
    }
}

fn span_from_range(range: &Range, token_type: u32) -> Option<TokenSpan> {
    if range.start.line != range.end.line {
        return None;
    }
    let length = range.end.character.saturating_sub(range.start.character);
    if length == 0 {
        return None;
    }
    Some(TokenSpan {
        line: range.start.line,
        start: range.start.character,
        length,
        token_type,
    })
}

fn token_type_for_symbol(symbol: &IndexedSymbol) -> u32 {
    match symbol.kind {
        IndexedSymbolKind::Label => 4,
        IndexedSymbolKind::Screen => 4,
        IndexedSymbolKind::Transform => 2,
        IndexedSymbolKind::Style => 4,
        IndexedSymbolKind::Image => 4,
        IndexedSymbolKind::Variable | IndexedSymbolKind::Assignment | IndexedSymbolKind::Import => 1,
        IndexedSymbolKind::Function => 2,
        IndexedSymbolKind::Class => 3,
    }
}

fn token_type_for_reference(reference: &IndexedReference) -> u32 {
    match reference.language {
        SymbolLanguage::Python => 1,
        SymbolLanguage::Renpy => 1,
    }
}

fn collect_keyword_tokens(document: &crate::parse::ParsedDocument, spans: &mut Vec<TokenSpan>) {
    if !matches!(document.language, crate::source::DocumentLanguage::Renpy) {
        return;
    }
    const KEYWORDS: &[&str] = &[
        "label", "screen", "transform", "style", "image", "default", "define", "jump", "call",
        "scene", "show", "hide", "with", "python", "init", "menu", "if", "elif", "else", "while",
        "return",
    ];

    for (line_index, line) in document.source.text.lines().enumerate() {
        for keyword in KEYWORDS {
            let mut offset = 0usize;
            while let Some(found) = line[offset..].find(keyword) {
                let start = offset + found;
                let end = start + keyword.len();
                let before_ok = start == 0
                    || !line[..start]
                        .chars()
                        .last()
                        .is_some_and(|ch| ch.is_alphanumeric() || ch == '_');
                let after_ok = end == line.len()
                    || !line[end..]
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_alphanumeric() || ch == '_');
                if before_ok && after_ok {
                    spans.push(TokenSpan {
                        line: line_index as u32,
                        start: start as u32,
                        length: keyword.len() as u32,
                        token_type: 0,
                    });
                }
                offset = end;
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "renpyfmt".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                document_formatting_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens_legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "renpyfmt LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.refresh_document(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.refresh_document(params.text_document.uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.workspace.write().await.remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let Some((path, text)) = self.document_text(&params.text_document.uri).await else {
            return Ok(None);
        };
        let input_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let formatted = match format_source(input_dir, &path, &text, &self.python_format_config) {
            Ok(formatted) => formatted,
            Err(err) => {
                self.client
                    .log_message(MessageType::ERROR, format!("formatting failed: {err:#}"))
                    .await;
                return Ok(None);
            }
        };
        let end = full_document_end(&text);
        Ok(Some(vec![TextEdit {
            range: Range::new(Position::new(0, 0), end),
            new_text: formatted,
        }]))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let workspace = self.workspace.read().await;
        let Some(document) = workspace.document(&params.text_document.uri) else {
            return Ok(None);
        };
        let symbols = document.symbols.document_symbols(&params.text_document.uri);
        Ok(Some(DocumentSymbolResponse::Flat(symbols)))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let workspace = self.workspace.read().await;
        Ok(Some(workspace.workspace_symbols(&params.query)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let workspace = self.workspace.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        Ok(workspace
            .goto_definition(uri, position)
            .map(|location| GotoDefinitionResponse::Scalar(location)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let workspace = self.workspace.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(document) = workspace.document(uri) else {
            return Ok(None);
        };
        let Some(contents) = document.symbols.hover(uri, position) else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: contents,
            }),
            range: None,
        }))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let workspace = self.workspace.read().await;
        let Some(document) = workspace.document(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(compute_semantic_tokens(&document).into()))
    }
}

pub async fn run_server() {
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin(), stdout(), socket).serve(service).await;
}

fn file_path_from_uri(uri: &Url) -> Option<PathBuf> {
    uri.to_file_path().ok()
}

fn full_document_end(text: &str) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position::new(line, col)
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use serde_json::json;
    use tower::Service;
    use tower::ServiceExt;
    use tower_lsp::jsonrpc::{Request, Response};

    use super::*;
    use crate::parse::{ParseOptions, parse_document};
    use crate::source::SourceDocument;

    fn initialize_request(id: i64) -> Request {
        Request::build("initialize")
            .params(json!({"capabilities":{}}))
            .id(id)
            .finish()
    }

    #[tokio::test(flavor = "current_thread")]
    async fn initialize_reports_capabilities() {
        let (mut service, _) = LspService::new(Backend::new);

        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize_request(1))
            .await
            .unwrap();

        let ok = Response::from_ok(
            1.into(),
            json!({
                "capabilities": {
                    "textDocumentSync": 1,
                    "documentFormattingProvider": true,
                    "definitionProvider": true,
                    "hoverProvider": true,
                    "documentSymbolProvider": true,
                    "workspaceSymbolProvider": true,
                    "semanticTokensProvider": {
                        "full": true,
                        "legend": {
                            "tokenModifiers": [],
                            "tokenTypes": ["keyword", "variable", "function", "class", "namespace"]
                        }
                    }
                },
                "serverInfo": {
                    "name": "renpyfmt",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        );
        assert_eq!(response, Some(ok));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn did_open_publishes_diagnostics() {
        let (mut service, mut socket) = LspService::new(Backend::new);
        let _ = service
            .ready()
            .await
            .unwrap()
            .call(initialize_request(1))
            .await
            .unwrap();

        let uri = Url::parse("file:///tmp/open_bad.rpy").unwrap();
        let open = Request::build("textDocument/didOpen")
            .params(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "renpy",
                    "version": 1,
                    "text": "python:\n    def broken(\n        pass\n"
                }
            }))
            .finish();

        let response = service.ready().await.unwrap().call(open).await.unwrap();
        assert!(response.is_none());

        let message = socket.next().await.expect("client notification");
        assert_eq!(message.method(), "textDocument/publishDiagnostics");
        let params = message.params().cloned().expect("diagnostics params");
        let diagnostics = params
            .get("diagnostics")
            .and_then(|value| value.as_array())
            .expect("diagnostic array");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn semantic_tokens_include_keywords_and_python_references() {
        let path = PathBuf::from("/tmp/semantic_tokens.rpy");
        let source = SourceDocument::new(
            Url::from_file_path(&path).ok(),
            path,
            concat!(
                "init python:\n",
                "    def day_planner():\n",
                "        return 1\n",
                "\n",
                "label start:\n",
                "    $ day_planner()\n",
            )
            .into(),
        );
        let document = parse_document(source, ParseOptions);
        let tokens = compute_semantic_tokens(&document);

        assert!(!tokens.data.is_empty());
        assert!(tokens.data.iter().any(|token| token.token_type == 0));
        assert!(tokens.data.iter().any(|token| token.token_type == 2));
        assert!(tokens.data.iter().any(|token| token.token_type == 1));
    }
}

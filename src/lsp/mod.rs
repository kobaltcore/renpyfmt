use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{stdin, stdout};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

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
                    "workspaceSymbolProvider": true
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
}

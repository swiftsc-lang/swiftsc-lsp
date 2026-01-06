/// SwiftSC-Lang Language Server Protocol Implementation
///
/// Provides IDE support for SwiftSC-Lang smart contracts
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct SCLanguageServer {
    pub client: Client,
    pub documents: Mutex<HashMap<Url, String>>,
}

impl SCLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for SCLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions::default()),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "SwiftSC-Lang LSP server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let content = params.text_document.text;
        self.documents
            .lock()
            .unwrap()
            .insert(params.text_document.uri.clone(), content.clone());
        self.run_diagnostics(params.text_document.uri, content)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            let content = change.text;
            self.documents
                .lock()
                .unwrap()
                .insert(params.text_document.uri.clone(), content.clone());
            self.run_diagnostics(params.text_document.uri, content)
                .await;
        }
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(None)
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(
                "SwiftSC Smart Contract Element\n\nSecurity diagnostics active.".to_string(),
            )),
            range: None,
        }))
    }
}

impl SCLanguageServer {
    async fn run_diagnostics(&self, uri: Url, content: String) {
        let mut diagnostics = Vec::new();

        match swiftsc_frontend::parse(&content) {
            Ok(program) => {
                let mut security_analyzer = swiftsc_analyzer::security::SecurityAnalyzer::new();
                security_analyzer.analyze_program(&program);

                for warning in security_analyzer.get_warnings() {
                    let message = warning.message();
                    let span = match warning {
                        swiftsc_analyzer::security::SecurityWarning::PotentialReentrancy { span, .. } => span,
                        swiftsc_analyzer::security::SecurityWarning::UninitializedVariable { span, .. } => span,
                        swiftsc_analyzer::security::SecurityWarning::UncheckedArithmetic { span, .. } => span,
                        swiftsc_analyzer::security::SecurityWarning::PotentialOverflow { span, .. } => span,
                    };

                    let (line, col) = span.to_lsp_pos();
                    diagnostics.push(Diagnostic {
                        range: Range::new(Position::new(line, col), Position::new(line, col + 1)),
                        severity: Some(DiagnosticSeverity::WARNING),
                        message,
                        source: Some("SwiftSC Security".to_string()),
                        ..Default::default()
                    });
                }
            }
            Err(e) => {
                let (message, span) = match e {
                    swiftsc_frontend::parser::ParseError::Expected(expected, found, span) => {
                        (format!("Expected {}, found {:?}", expected, found), span)
                    }
                    swiftsc_frontend::parser::ParseError::UnexpectedToken(token, span) => {
                        (format!("Unexpected token: {:?}", token), span)
                    }
                    _ => (format!("Parser Error: {:?}", e), swiftsc_frontend::ast::Span::new(1, 1)),
                };

                let (line, col) = span.to_lsp_pos();
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(line, col), Position::new(line, col + 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message,
                    source: Some("SwiftSC Parser".to_string()),
                    ..Default::default()
                });
            }
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

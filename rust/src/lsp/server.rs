use anyhow::Context;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use super::diagnostics::to_lsp_diagnostic;
use crate::checker::FileChecker;

pub struct MethodRayServer {
    client: Client,
    documents: RwLock<HashMap<Url, String>>,
}

impl MethodRayServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: RwLock::new(HashMap::new()),
        }
    }

    async fn check_document(&self, uri: Url) {
        let documents = self.documents.read().await;

        if let Some(source) = documents.get(&uri) {
            match self.run_type_check(&uri, source).await {
                Ok(diagnostics) => {
                    self.client
                        .publish_diagnostics(uri.clone(), diagnostics, None)
                        .await;
                }
                Err(e) => {
                    self.client
                        .log_message(MessageType::ERROR, format!("Type check failed: {}", e))
                        .await;
                }
            }
        }
    }

    async fn run_type_check(&self, uri: &Url, source: &str) -> anyhow::Result<Vec<Diagnostic>> {
        // Convert URI to file path
        let file_path = uri
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("Invalid file URI: {}", uri))?;

        // Create a temporary file for checking
        let temp_dir = tempfile::tempdir()?;
        let temp_file = temp_dir.path().join("temp.rb");
        std::fs::write(&temp_file, source)?;

        // Run type check using FileChecker
        let checker = FileChecker::new().with_context(|| "Failed to create FileChecker")?;

        let methodray_diagnostics = checker
            .check_file(&temp_file)
            .with_context(|| format!("Failed to check file: {}", file_path.display()))?;

        // Convert to LSP diagnostics
        let lsp_diagnostics = methodray_diagnostics
            .iter()
            .map(to_lsp_diagnostic)
            .collect();

        Ok(lsp_diagnostics)
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for MethodRayServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "MethodRay LSP server initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.documents.write().await.insert(uri.clone(), text);
        self.check_document(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Some(change) = params.content_changes.first() {
            let text = change.text.clone();
            self.documents.write().await.insert(uri.clone(), text);
            // Note: In production, we'd want debouncing here
            // self.check_document(uri).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.check_document(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);

        // Clear diagnostics
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| MethodRayServer::new(client));

    Server::new(stdin, stdout, socket).serve(service).await;
}

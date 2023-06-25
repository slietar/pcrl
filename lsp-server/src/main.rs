use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        // eprintln!("Client capabilities: {:#?}", params.capabilities);

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // workspace: Some(WorkspaceServerCapabilities {
                //     workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                //         supported: Some(true),
                //         change_notifications: Some(OneOf::Left(true)),
                //     }),
                //     file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                //         ..Default::default()
                //     }),
                // }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client.publish_diagnostics(params.text_document.uri, produce_diagnostics(&params.text_document.text), Some(params.text_document.version)).await;

        // eprintln!("{:#?}", params);

        // self.client
        //     .log_message(MessageType::INFO, "file opened!")
        //     .await;

        // self.client.publish_diagnostics(params.text_document.uri, vec![
        //     Diagnostic {
        //         message: "Problem here".to_string(),
        //         range: Range {
        //             end: Position { character: 5, line: 1 },
        //             start: Position { character: 3, line: 1 }
        //         },
        //         severity: Some(DiagnosticSeverity::ERROR),
        //         ..Default::default()
        //     }
        // ], Some(params.text_document.version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client.publish_diagnostics(params.text_document.uri, produce_diagnostics(&params.content_changes[0].text), Some(params.text_document.version)).await;
    }
}


fn produce_diagnostics(text: &str) -> Vec<Diagnostic> {
    let result = pcrl::parse::<pcrl::counters::LspUtf16>(text);

    result.errors
        .iter()
        .map(|error| {
            Diagnostic {
                message: format!("{:?}", error.value),
                range: Range {
                    end: Position {
                        character: error.span.1.counter.column as u32,
                        line: error.span.1.counter.line as u32,
                    },
                    start: Position {
                        character: error.span.0.counter.column as u32,
                        line: error.span.0.counter.line as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                ..Default::default()
            }
        })
        .collect::<Vec<_>>()
}


#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}

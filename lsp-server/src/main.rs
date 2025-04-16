use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    results: DashMap<Url, pcrl::ParseResult<pcrl::indexers::LineColumnIndex>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            results: DashMap::new(),
        }
    }

    async fn on_change(&self, uri: &Url, version: i32, text: &str) {
        let result = pcrl::parse::<pcrl::indexers::LspUtf16>(text);

        let diagnostics = result.errors
            .iter()
            .map(|error| {
                Diagnostic {
                    message: format!("{:?}", error.value),
                    range: Range {
                        end: Position {
                            character: error.span.1.index.column as u32,
                            line: error.span.1.index.line as u32,
                        },
                        start: Position {
                            character: error.span.0.index.column as u32,
                            line: error.span.0.index.line as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();

        self.results.insert(uri.clone(), result);
        self.client.publish_diagnostics(uri.clone(), diagnostics, Some(version)).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        // eprintln!("Client capabilities: {:#?}", params.capabilities);

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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
        self.on_change(&params.text_document.uri, params.text_document.version, &params.text_document.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(&params.text_document.uri, params.text_document.version, &params.content_changes[0].text).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.results.remove(&params.text_document.uri);
    }

    // TextDocumentPositionParams { text_document: TextDocumentIdentifier { uri: Url { scheme: "file", cannot_be_a_base: false, username: "", password: None, host: None, port: None, path: "/Users/simon/Downloads/Untitled-1.txt", query: None, fragment: None } }, position: Position { line: 6, character: 0 } }
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        // let contents = HoverContents::Scalar(MarkedString::String("Hello, world!".to_owned()));
        let result = self.results.get(&params.text_document_position_params.text_document.uri);

        let position = params.text_document_position_params.position;

        match result {
            Some(result) => {
                let find_result = pcrl::find(result.value(), pcrl::indexers::LineColumnIndex {
                    line: position.line as usize,
                    column: position.character as usize,
                }, false);

                match find_result {
                    Some(pcrl::FindResult::MapKey { entry, path }) => {
                        let mut contents = vec![MarkedString::String(format!("Key: {}", entry.key.value))];
                        contents.extend(
                            entry.context.comments
                                .iter()
                                .map(|comment| MarkedString::String(comment.contents.value.clone()))
                                // .collect::<Vec<_>>()
                        );

                        Ok(Some(Hover {
                            // contents: HoverContents::Scalar(MarkedString::String(format!("Key: {}\n{}", entry.key.value, comments))),
                            contents: HoverContents::Array(contents),
                            range: None,
                        }))
                    },
                    Some(pcrl::FindResult::Value { object, path }) => {
                        Ok(Some(Hover {
                            contents: HoverContents::Scalar(MarkedString::String(format!("Path: {:?}", path))),
                            range: None,
                        }))
                    },
                    None => {
                        Ok(Some(Hover {
                            contents: HoverContents::Scalar(MarkedString::String("Not found".to_string())),
                            range: None,
                        }))
                    },
                }

                // eprintln!("{:#?}", x);

                // Ok(Some(Hover {
                //     contents,
                //     range: None,
                // }))
            },
            None => {
                Ok(None)
            },
        }
    }
}


#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}

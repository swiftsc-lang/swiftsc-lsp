use swiftsc_lsp::lsp::SCLanguageServer;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(SCLanguageServer::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}

//! LSP server binary entry point

use methodray_core::lsp;

#[tokio::main]
async fn main() {
    lsp::run_server().await;
}

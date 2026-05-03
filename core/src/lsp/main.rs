//! LSP server binary entry point
//!
//! [DEPRECATED] methodray-lsp will be removed in a future version.

use methodray_core::lsp;

#[tokio::main]
async fn main() {
    eprintln!("WARNING: 'methodray-lsp' is deprecated and will be removed in a future version.");
    lsp::run_server().await;
}

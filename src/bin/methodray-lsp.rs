use methodray::lsp;

#[tokio::main]
async fn main() {
    lsp::run_server().await;
}

//! fisher-server entry point: boot the Axum app on :7200 with logging.

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();

    let addr = "0.0.0.0:7200";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind :7200");

    tracing::info!("fisher-server listening on http://{addr}");
    fisher_server::serve(listener).await;
}

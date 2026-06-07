use axum::{Router, routing::get};
use tracing::info;

mod handler;
mod readability_parser;
mod tts;

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    tracing_subscriber::fmt().init();

    let parser = readability_parser::spawn();

    let router = Router::new()
        .route("/", get(handler::root))
        .route("/widget.js", get(async || "some js code"))
        .with_state(handler::AppState { parser });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    info!("Server started on {}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.unwrap();
}

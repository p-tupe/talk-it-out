use std::env;

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
        .route("/test_page", get(handler::test_page))
        .with_state(handler::AppState { parser });

    let port = env::var("PORT").unwrap_or("8080".into());
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    info!("Server started on {}", addr);
    axum::serve(listener, router).await.unwrap();
}

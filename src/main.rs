use axum::{Router, extract::Query, routing::get};
use serde::Deserialize;
use tracing::info;

mod reader;
mod tts;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let router = Router::new().route("/", get(handler));
    let listener = tokio::net::TcpListener::bind("localhost:8080")
        .await
        .unwrap();

    info!("Server started on :8080");
    axum::serve(listener, router).await.unwrap();
}

#[derive(Deserialize)]
struct Params {
    url: String,
}

async fn handler(params: Query<Params>) -> String {
    info!("Received request");
    let content = reader::get_content(&params.url);
    tts::stream_audio(&content)
}

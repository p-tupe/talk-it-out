use axum::{Router, extract::Query, routing::get};
use readability_js::Readability;
use reqwest::StatusCode;
use scraper::Html;
use serde::Deserialize;
use tracing::{error, info};

#[derive(Deserialize)]
struct Params {
    url: String,
}

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

async fn handler(params: Query<Params>) -> (StatusCode, String) {
    info!("Received request");

    let resp = match reqwest::get(&params.url).await {
        Ok(v) => v,
        Err(e) => {
            error!("{e}");
            return (StatusCode::BAD_REQUEST, e.to_string());
        }
    };

    let html = match resp.text().await {
        Ok(v) => v,
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    };

    // TODO: lift readability up, make it  a singleton
    let readability = Readability::new().unwrap();
    let article = match readability.parse(&html) {
        Ok(v) => v,
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    };

    (
        StatusCode::OK,
        Html::parse_fragment(&article.content)
            .root_element()
            .text()
            .into_iter()
            .collect::<String>(),
    )
}

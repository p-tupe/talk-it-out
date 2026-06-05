use std::sync::mpsc;

use axum::{
    Router,
    extract::{Query, Request, State},
    routing::get,
};
use readability_js::{Readability, ReadabilityError};
use reqwest::StatusCode;
use scraper::Html;
use serde::Deserialize;
use tokio::sync::oneshot;
use tracing::{error, info};

#[derive(Deserialize)]
struct Params {
    url: String,
}

#[derive(Clone)]
struct AppState {
    readability_parser: mpsc::Sender<Payload>,
}

struct Payload {
    html_doc: String,
    sender_chan: oneshot::Sender<Result<String, ReadabilityError>>,
}

#[allow(dead_code)]
struct AppError {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let readability_parser = spawn_readability_parser();

    let router = Router::new()
        .route("/", get(handler))
        .route("/widget.js", get(async || "some js code"))
        .with_state(AppState {
            readability_parser: readability_parser,
        });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    info!("Server started on :8080");
    axum::serve(listener, router).await.unwrap();
}

#[axum::debug_handler]
async fn handler(
    params: Query<Params>,
    state: State<AppState>,
    req: Request,
) -> Result<String, (StatusCode, &'static str)> {
    info!("{} {}", req.method(), req.uri());

    let resp = match reqwest::get(&params.url).await {
        Ok(v) => v,
        Err(e) => {
            error!("Error fetching url: {e}");
            return Err((
                StatusCode::BAD_REQUEST,
                "Unexpected error while fetching url",
            ));
        }
    };

    let html = match resp.text().await {
        Ok(v) => v,
        Err(e) => {
            error!("Error reading response: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unexpected error reading url response",
            ));
        }
    };

    let (os_tx, os_rx) = oneshot::channel();
    state
        .readability_parser
        .send(Payload {
            html_doc: html,
            sender_chan: os_tx,
        })
        .unwrap();

    let article = match os_rx.await.unwrap() {
        Ok(article) => article,
        Err(e) => {
            error!("Error parsing html: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unexpected error parsing html doc",
            ));
        }
    };

    Ok(article)
}

fn spawn_readability_parser() -> mpsc::Sender<Payload> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let readability = Readability::new().unwrap();
        let mut itr = rx.iter();

        loop {
            let Payload {
                html_doc,
                sender_chan,
            } = itr.next().unwrap();

            let article = match readability.parse(&html_doc) {
                Ok(v) => v,
                Err(e) => {
                    sender_chan.send(Err(e)).unwrap();
                    return;
                }
            };

            let content = Html::parse_fragment(&article.content)
                .root_element()
                .text()
                .into_iter()
                .collect::<String>();

            sender_chan.send(Ok(content)).unwrap();
        }
    });

    tx
}

use std::sync::mpsc;

use axum::{
    extract::{Query, Request, State},
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::oneshot;
use tracing::{error, info};

use crate::readability_parser::Payload;

#[derive(Deserialize)]
pub struct Params {
    url: String,
}

#[derive(Clone)]
pub struct AppState {
    pub parser: mpsc::Sender<Payload>,
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("{}", self.0);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unexpected error: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[axum::debug_handler]
pub async fn root(
    params: Query<Params>,
    state: State<AppState>,
    req: Request,
) -> Result<String, AppError> {
    info!("{} {}", req.method(), req.uri());

    let resp = reqwest::get(&params.url).await?;
    let status = resp.status();
    let text = resp.text().await?;

    if status != StatusCode::OK {
        return Err(AppError(anyhow::anyhow!(status)));
    }

    let (os_tx, os_rx) = oneshot::channel();

    state.parser.send(Payload {
        html_doc: text,
        sender_chan: os_tx,
    })?;

    let article = os_rx.await??;

    Ok(article)
}

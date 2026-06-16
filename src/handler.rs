use std::sync::mpsc;

use axum::{
    extract::{Query, Request, State},
    response::{Html, IntoResponse, Response},
};
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::oneshot;
use tracing::{error, info};

use crate::readability_parser::Payload;
use crate::tts::generate;

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

pub async fn root(
    params: Query<Params>,
    state: State<AppState>,
    req: Request,
) -> impl IntoResponse {
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

    Ok(generate(article).await?)
}

pub async fn test_page() -> Html<String> {
    Html(
        "
<input type='text' id='url-input' placeholder='Enter URL here'> </input>
<button type='button' onclick='getAudio()'>Go</button>

<audio controls style='display:none'>
  <source src='' type='audio/mpeg'>
  Your browser does not support the audio tag.
</audio>

<script>
function getAudio() {
  const url = document.querySelector('#url-input').value;

  const audio = document.querySelector('audio');
  audio.style.display = 'block';
  const source = audio.querySelector('source');
  source.src = 'http://127.0.0.1:8080?url=' + url;
  audio.load();
}
</script>
"
        .to_string(),
    )
}

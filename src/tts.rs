//! TTS interface for generating audio

use anyhow::{Error, Result, anyhow};
use msedge_tts::{
    tts::{SpeechConfig, client::tokio_runtime::connect_async},
    voice::tokio_runtime::get_voices_list_async,
};
use std::time::Instant;

pub async fn generate(content: &str) -> Result<Vec<f32>> {
    let voices = get_voices_list_async().await.map_err(Error::msg)?;

    let Some(voice) = voices
        .iter()
        .find(|v| v.short_name == Some("en-US-MichelleNeural".into()))
    else {
        return Err(anyhow!("Could not find voice"));
    };

    println!("choose '{}' to synthesize...", voice.name);
    let config = SpeechConfig::from(voice);
    let mut tts = connect_async().await.map_err(Error::msg)?;
    let start = Instant::now();
    let audio = tts.synthesize(content, &config).await.map_err(Error::msg)?;
    println!("Done {:?}", Instant::now() - start);

    println!("play audio...");
    let handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
    let player = rodio::Player::connect_new(handle.mixer());

    let decoder = rodio::decoder::Decoder::new(std::io::Cursor::new(audio.audio_bytes)).unwrap();

    player.append(decoder);
    player.sleep_until_end();
    println!("play audio done.");

    Ok(vec![])
}

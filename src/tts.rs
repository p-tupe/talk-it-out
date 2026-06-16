//! TTS interface for generating audio

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Error, Result, anyhow};
use msedge_tts::{
    tts::{
        SpeechConfig,
        stream::{SynthesizedResponse, msedge_tts_split},
    },
    voice::tokio_runtime::get_voices_list_async,
};

pub async fn generate(content: String) -> Result<Vec<u8>> {
    let voices = get_voices_list_async().await.map_err(Error::msg)?;

    let Some(voice) = voices
        .iter()
        .find(|v| v.short_name == Some("en-US-MichelleNeural".into()))
    else {
        return Err(anyhow!("Could not find voice"));
    };

    let config = SpeechConfig::from(voice);
    let (mut sender, mut reader) = msedge_tts_split().unwrap();
    let signal = Arc::new(AtomicBool::new(false));
    let end = signal.clone();

    sender.send(&content, &config).unwrap();
    end.store(true, Ordering::Relaxed);

    let mut audio_bytes = vec![];
    loop {
        if !reader.can_read() {
            break;
        }

        let audio = reader.read().unwrap();
        if let Some(chunk) = audio {
            if let SynthesizedResponse::AudioBytes(b) = chunk {
                audio_bytes.extend(b);
            };
        };
    }

    Ok(audio_bytes)
}

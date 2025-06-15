use arma_rs::{Context, ContextState};
use reqwest::Client;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::io::Cursor;
use std::sync::mpsc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::settings::Settings;

#[allow(clippy::needless_pass_by_value)]
pub fn cmd_speak(ctx: Context, data: String, pan: f32, volume: f32) {
    let settings = ctx
        .global()
        .get::<Settings>()
        .unwrap_or_else(|| {
            ctx.global().set(Settings::default());
            ctx.global().get::<Settings>().unwrap()
        })
        .clone();
    ctx.global().get::<Runtime>().unwrap().spawn(openai(
        settings,
        Uuid::parse_str(&data).unwrap(),
        pan,
        volume,
    ));
}

async fn openai(settings: Settings, id: Uuid, pan: f32, volume: f32) {
    println!("Requesting audio for ID: {id}");
    // Create a new HTTP client
    let client = Client::new();

    // Get the host from settings
    let Some(host) = settings.get("HOST") else {
        eprintln!("Error: HOST not found in settings");
        return;
    };
    let host = host.as_str().unwrap().to_string();
    if !host.starts_with("http://") && !host.starts_with("https://") {
        eprintln!("Error: Invalid host URL: {host}");
        return;
    }
    println!("Using host: {host}");
    // Make a GET request to the /speak/{id} endpoint
    let response = match client.get(format!("{host}/speak/{id}")).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Error: Failed to send request: {e}");
            return;
        }
    };

    // Check if the request was successful
    if !response.status().is_success() {
        eprintln!(
            "Error: Received non-success status code: {}",
            response.status()
        );
        return;
    }

    // Get the bytes from the response
    let Ok(mp3_data) = response.bytes().await else {
        eprintln!("Error: Failed to read response bytes");
        return;
    };

    // Play the MP3 file
    match play_audio_with_rodio(mp3_data.to_vec(), pan, volume) {
        Ok(()) => {
            println!("Played audio successfully");
        }
        Err(e) => {
            eprintln!("Error: Failed to play audio: {e}");
        }
    }
}

fn play_audio_with_rodio(data: Vec<u8>, pan: f32, volume: f32) -> Result<(), String> {
    // Create cursor from the MP3 data
    let cursor = Cursor::new(data);

    // Initialize the audio output stream
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| format!("Failed to get audio output stream: {e}"))?;

    // Create a sink to play the audio
    let sink =
        Sink::try_new(&stream_handle).map_err(|e| format!("Failed to create audio sink: {e}"))?;

    // Decode the MP3 data
    let source = Decoder::new(cursor).map_err(|e| format!("Failed to decode audio data: {e}"))?;

    // Add the audio to the sink
    sink.append(pan_volume(source, pan, volume));

    println!("Playing audio with pan: {pan}, volume: {volume}");

    // Start playback
    sink.play();

    // Create a channel to signal completion
    let (tx, rx) = mpsc::channel();

    // Set up a timeout in case the audio fails to play
    let timeout_duration = Duration::from_secs(60); // Adjust as needed

    // Start a thread to wait for audio completion
    std::thread::spawn(move || {
        // This blocks until the sink is empty and the audio has finished
        sink.sleep_until_end();
        // Signal that playback is complete
        let _ = tx.send(());
    });

    // Wait for playback to complete or timeout
    if rx.recv_timeout(timeout_duration).is_ok() {
        println!("Audio playback completed normally");
        Ok(())
    } else {
        eprintln!("Audio playback timed out");
        Err("Audio playback timed out or was interrupted".to_string())
    }

    // stream is dropped here, which automatically closes the audio device
}

pub struct PanVolumeSource<S>
where
    S: Source<Item = i16>,
{
    input: S,
    pan: f32,    // -1.0 (left) to 1.0 (right)
    volume: f32, // 1.0 = normal
    left: bool,
    mono_repeat: i16,
}

impl<S> Iterator for PanVolumeSource<S>
where
    S: Source<Item = i16>,
{
    type Item = i16;

    #[allow(clippy::cast_possible_truncation)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.input.channels() == 1 {
            if self.left {
                let sample = self.input.next()?;
                // Apply left pan and volume
                self.left = false;
                self.mono_repeat = sample;
                return Some((sample as f32 * (1.0 - self.pan) * self.volume) as i16);
            }
            // Apply right pan and volume
            self.left = true;
            let sample = self.mono_repeat;
            return Some((sample as f32 * (1.0 + self.pan) * self.volume) as i16);
        }
        let sample = self.input.next()?;
        if self.left {
            // Apply left pan and volume
            self.left = false;
            return Some((sample as f32 * (1.0 - self.pan) * self.volume) as i16);
        }
        // Apply right pan and volume
        self.left = true;
        Some((sample as f32 * (1.0 + self.pan) * self.volume) as i16)
    }
}

impl<S> Source for PanVolumeSource<S>
where
    S: Source<Item = i16>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
}

/// Helper to wrap a source with pan and volume
pub fn pan_volume<S>(source: S, pan: f32, volume: f32) -> PanVolumeSource<S>
where
    S: Source<Item = i16>,
{
    PanVolumeSource {
        input: source,
        pan,
        volume,
        left: true,
        mono_repeat: 0,
    }
}

#[cfg(test)]
#[test]
fn test_pan_volume() {
    // Decode "womp.mp3" into a source
    let data = include_bytes!("../../../womp.mp3").to_vec();
    play_audio_with_rodio(data, 1.0, 0.8).expect("Failed to play audio with pan and volume");
}

#[cfg(test)]
#[test]
fn test_pan_volume_mono() {
    // Decode "mono.mp3" into a source
    let data = include_bytes!("../../../mono.mp3").to_vec();
    play_audio_with_rodio(data, 1.0, 0.8).expect("Failed to play audio with pan and volume");
}

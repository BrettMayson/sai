use arma_rs::{Context, ContextState, Group};
use reqwest::Client;
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::mpsc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::settings::Settings;

pub fn group() -> Group {
    let mut group = Group::new().command("openai", cmd_openai);
    #[cfg(feature = "local")]
    {
        group = group.command("local", cmd_local);
    }
    group
}

#[cfg(feature = "local")]
pub fn cmd_local(ctx: Context, data: String) -> Result<(), String> {
    use tts::Tts;
    println!("speaking locally: {}", data);
    std::thread::spawn(move || {
        let tts = ctx.global().get::<Mutex<Tts>>().unwrap_or_else(|| {
            ctx.global()
                .set::<Mutex<Tts>>(Mutex::new(Tts::default().unwrap()));
            ctx.global().get::<Mutex<Tts>>().unwrap()
        });
        if let Err(e) = tts.lock().unwrap().speak(data, false) {
            eprintln!("Error: Failed to speak: {}", e);
        } else {
            println!("Spoken successfully");
        };
    });
    Ok(())
}

pub fn cmd_openai(ctx: Context, data: String) {
    let settings = ctx
        .global()
        .get::<Settings>()
        .unwrap_or_else(|| {
            ctx.global().set(Settings::default());
            ctx.global().get::<Settings>().unwrap()
        })
        .clone();
    ctx.global()
        .get::<Runtime>()
        .unwrap()
        .spawn(openai(settings, Uuid::parse_str(&data).unwrap()));
}

async fn openai(settings: Settings, id: Uuid) {
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
    match play_audio_with_rodio(mp3_data.to_vec()) {
        Ok(()) => {
            println!("Played audio successfully");
        }
        Err(e) => {
            eprintln!("Error: Failed to play audio: {e}");
        }
    }
}

fn play_audio_with_rodio(data: Vec<u8>) -> Result<(), String> {
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
    sink.append(source);

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

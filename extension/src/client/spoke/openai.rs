use std::path::{Path, PathBuf};

use arma_rs::{Context, ContextState};
use reqwest::multipart::{Form, Part};
use tokio::{io::AsyncReadExt, runtime::Runtime};

use crate::{settings::Settings, TokioContext};

pub fn spoke(ctx: Context, path: PathBuf, callsign: String) {
    let settings = ctx.global().get::<Settings>().unwrap_or_else(|| {
        ctx.global().set(Settings::default());
        ctx.global().get::<Settings>().unwrap()
    }).clone();
    ctx.global().get::<Runtime>().unwrap().spawn(async move {
        let Ok(mut file) = tokio::fs::File::open(&path).await else {
            eprintln!("Error opening file");
            return;
        };
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .expect("Failed to read file");
        let file_name = Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav");

        let Some(host) = settings.get("HOST") else {
            eprintln!("HOST not found in settings");
            return;
        };
        let response = match reqwest::Client::new()
            .post(format!("{host}/spoke"))
            .multipart(
                Form::new().part(
                    "audio",
                    Part::bytes(buffer)
                        .file_name(file_name.to_string())
                        .mime_str("audio/wav")
                        .expect("Failed to set MIME type"),
                ),
            )
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                eprintln!("Error sending request: {err}");
                return;
            }
        };

        if response.status().is_success() {
            println!("Successfully sent WAV file");
            let response = response.text().await.expect("Failed to read response");
            println!("Response: {response}");
            if let Err(e) = TokioContext::get()
                .unwrap()
                .callback_data("sai", "spoke", (callsign, response))
            {
                eprintln!("Error sending callback data: {e}");
            }
        } else {
            println!("Failed to send WAV file: {}", response.status());
            println!(
                "Error: {}",
                response
                    .text()
                    .await
                    .expect("Failed to read error response")
            );
        }
    });
}

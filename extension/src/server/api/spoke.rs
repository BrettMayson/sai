use axum::{extract::Multipart, http::StatusCode, response::IntoResponse};
use openai_api_rs::v1::audio::{AudioTranscriptionRequest, WHISPER_1};

use crate::{TokioContext, server::openai_client};

pub async fn handler(mut multipart: Multipart) -> impl IntoResponse {
    println!("Received multipart request");
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        let Some(ctx) = TokioContext::get() else {
            eprintln!("Failed to get TokioContext");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get context".to_string(),
            );
        };

        if name == "audio" {
            // Get the file content as bytes
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                let req =
                    AudioTranscriptionRequest::new_bytes(data.to_vec(), WHISPER_1.to_string())
                        .language("en".to_string());
                let req_json = req.clone().response_format("json".to_string());
                let mut client = match openai_client(&ctx) {
                    Ok(client) => client,
                    Err(err) => {
                        eprintln!("Error creating OpenAI client: {err}");
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Error creating client".to_string(),
                        );
                    }
                };
                let response = match client.audio_transcription(req_json).await {
                    Ok(result) => result,
                    Err(err) => {
                        eprintln!("Error processing audio: {err}");
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Error processing audio file".to_string(),
                        );
                    }
                };
                println!("{response:?}");
                return (StatusCode::OK, response.text);
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        "No audio file received".to_string(),
    )
}

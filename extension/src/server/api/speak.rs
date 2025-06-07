use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::fs;

// Handler for the /speak/{id} endpoint
pub async fn handler(Path(id): Path<String>) -> impl IntoResponse {
    println!("Received request to speak with ID: {id}");
    let audio_path = std::env::temp_dir().join(format!("sai_{id}.mp3"));

    // Check if the file exists
    if !audio_path.exists() {
        println!("Audio file not found: {}", audio_path.display());
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from(format!("Audio file with ID {id} not found")))
            .unwrap();
    }

    // Read the file
    match fs::read(&audio_path) {
        Ok(data) => {
            // Return the file with appropriate headers
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "audio/mpeg")
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{id}.mp3\""),
                )
                .body(Body::from(data))
                .unwrap()
        }
        Err(err) => {
            println!("Error reading audio file: {err}");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Error reading audio file"))
                .unwrap()
        }
    }
}

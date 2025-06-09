mod api;
mod commander;
mod response;

use arma_rs::{Context, ContextState, Group, Value};
use openai_api_rs::v1::api::OpenAIClient;
use tokio::runtime::Runtime;

use crate::{TokioContext, server::commander::CommanderPool, settings::Settings};

pub fn group() -> Group {
    Group::new()
        .group("commander", commander::group())
        .command("response", response::cmd_response)
        .command("spoke", cmd_spoke)
        .command("manual", cmd_manual)
        .command("start", cmd_start)
}

// start and axum api server on port 8521
fn cmd_start(ctx: Context) {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        println!("Starting server");
        let Some(tokio) = ctx.global().get::<Runtime>() else {
            eprintln!("Failed to get Tokio runtime");
            return;
        };
        println!("Starting server in background");
        tokio.spawn(async move {
            println!("Waiting for 500ms before starting server");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let listener = match tokio::net::TcpListener::bind("0.0.0.0:8521").await {
                Ok(listener) => listener,
                Err(e) => {
                    eprintln!("Error binding to port 8521: {e}");
                    return;
                }
            };
            println!("Server starting on port 8521");
            if let Err(e) = axum::serve(listener, api::app()).await {
                eprintln!("Error starting server: {e}");
            }
        });
    });
    CommanderPool::reset();
}

fn cmd_spoke(ctx: Context, callsign: String, text: String) -> Result<(), String> {
    handle_spoke(ctx, callsign, text, true)
}

fn cmd_manual(ctx: Context, callsign: String, text: String) -> Result<(), String> {
    handle_spoke(ctx, callsign, text, false)
}

fn handle_spoke(ctx: Context, callsign: String, text: String, speak: bool) -> Result<(), String> {
    println!("Manual command called");
    let Some(tokio) = ctx.global().get::<Runtime>() else {
        eprintln!("Failed to get Tokio runtime");
        return Err("Failed to get Tokio runtime".to_string());
    };
    let commander = CommanderPool::get(&callsign);
    tokio.spawn(async move {
        let ctx = TokioContext::get().unwrap();
        let response = match commander.input(&ctx, text).await {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Error processing chat: {err}");
                return;
            }
        };
        if speak {
            let Some(response) = response else {
                eprintln!("No response from Commander");
                return;
            };
            println!("Will speak: {response:?}");
            match commander::speak(&ctx, callsign, response).await {
                Ok(()) => {
                    println!("Ready for clients to speak");
                }
                Err(err) => {
                    eprintln!("Error speaking: {err}");
                }
            }
        } else {
            println!("Would have spoken: {response:?}");
        }
    });
    Ok(())
}

fn openai_client(ctx: &Context) -> Result<OpenAIClient, String> {
    match OpenAIClient::builder()
        .with_api_key({
            let settings = ctx.global().get::<Settings>().unwrap_or_else(|| {
                ctx.global().set(Settings::default());
                ctx.global().get::<Settings>().unwrap()
            });
            let Some(key) = settings.get("OPENAI_API_KEY") else {
                eprintln!("OPENAI_API_KEY not found in settings");
                return Err("OPENAI_API_KEY not found".to_string());
            };
            let Value::String(key) = key else {
                eprintln!("OPENAI_API_KEY is not a string");
                return Err("OPENAI_API_KEY is not a string".to_string());
            };
            key
        })
        .build()
    {
        Ok(client) => Ok(client),
        Err(err) => {
            eprintln!("Error creating OpenAI client: {err}");
            Err("Error creating client".to_string())
        }
    }
}

mod artillery;

use std::{
    collections::HashMap,
    env::temp_dir,
    sync::{Arc, LazyLock},
};

use arma_rs::{Context, Group};
use artillery::Artillery;
use dashmap::DashMap;
use openai_api_rs::v1::{
    audio::AudioSpeechRequest,
    chat_completion::{
        ChatCompletionMessage, ChatCompletionRequest, Content, FinishReason, MessageRole,
        ToolChoiceType,
    },
    common::GPT4_O_MINI,
};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::openai_client;

pub fn group() -> Group {
    Group::new().group("artillery", artillery::group())
}

pub struct CommanderPool {
    commanders: Arc<DashMap<String, Commander>>,
}

impl CommanderPool {
    pub fn get(callsign: &str) -> Commander {
        static POOL: LazyLock<CommanderPool> = LazyLock::new(|| CommanderPool {
            commanders: Arc::new(DashMap::new()),
        });
        POOL.commanders
            .entry(callsign.to_string())
            .or_insert_with(|| Commander::new(callsign.to_string()))
            .clone()
    }

    pub fn reset() {
        static POOL: LazyLock<CommanderPool> = LazyLock::new(|| CommanderPool {
            commanders: Arc::new(DashMap::new()),
        });
        POOL.commanders.clear();
    }
}

#[derive(Default, Clone)]
pub struct Commander {
    callsign: String,
    inner: Arc<Mutex<CommanderInner>>,
}

impl Commander {
    pub fn new(callsign: String) -> Self {
        Self {
            callsign,
            inner: Arc::new(Mutex::new(CommanderInner::default())),
        }
    }

    pub fn callsign(&self) -> &str {
        &self.callsign
    }

    #[allow(clippy::too_many_lines)]
    pub async fn input(&self, ctx: &Context, message: String) -> Result<Option<String>, String> {
        let mut inner = self.inner.lock().await;
        inner.history.push(ChatCompletionMessage {
            role: MessageRole::user,
            content: Content::Text(message),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        loop {
            let req = ChatCompletionRequest::new(GPT4_O_MINI.to_string(), {
                let mut history = vec![
                        ChatCompletionMessage {
                            role: MessageRole::system,
                            content: Content::Text(format!(
"You are {}, a mobile artillery commander supporting field units in a military operation.
Your job is to respond to requests from field units, providing support and information.
When responding be as consise as possible, use military lingo, short responses save lifes, don't waste time with generic advice.
The field team does not know what support is avilable, offer them the available options or pick the most logical.
The shorter the response the better. Do not over explain or ask unnecessary questions.
Give available rounds in this format, keeping it short and to the point: Shoelf has 1x HE, 2x Laser Guided
If the other party ends the conversation (having ended with out, for example), respond with 'no message', and no other text.
Upon every request, before asking for confirmation, check the available artillery and ETA using the tools to ensure you have the latest information and the order is possible.
Do not track the state of the rounds yourself, as other parties may change the state of the rounds.
Do at most 1 destructive action at a time, always confirm details with the user.", self.callsign())),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                    ];
                history.append(&mut inner.history.clone());
                history
            }).tools(vec![
                artillery::tool_artillery_fire_schema(),
                artillery::tool_artillery_available_schema(),
                artillery::tool_artillery_eta_schema(),
            ]).tool_choice(ToolChoiceType::Auto);
            let mut client = match openai_client(ctx) {
                Ok(client) => client,
                Err(err) => {
                    eprintln!("Error creating OpenAI client: {err}");
                    return Err("Error creating client".to_string());
                }
            };
            let result = match client.chat_completion(req).await {
                Ok(result) => result,
                Err(err) => {
                    eprintln!("Error processing chat: {err}");
                    return Err("Error processing chat".to_string());
                }
            };

            match result.choices[0].finish_reason {
                None
                | Some(
                    FinishReason::stop
                    | FinishReason::length
                    | FinishReason::content_filter
                    | FinishReason::null,
                ) => {
                    println!("finish: {:?}", result.choices[0].finish_reason);
                    println!("{:?}", result.choices[0].message.content);
                    if let Some(response) = &result.choices[0].message.content {
                        let response = response.replace("Over and out", "out");
                        inner.history.push(ChatCompletionMessage {
                            role: MessageRole::assistant,
                            content: Content::Text(response.to_string()),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        });
                        if response.to_lowercase().starts_with("no message") {
                            return Ok(None);
                        }
                        return Ok(Some(response));
                    }
                    return Ok(None);
                }
                Some(FinishReason::tool_calls) => {
                    println!("tool calls: {:?}", result.choices[0].message.tool_calls);
                    for tool_call in result.choices[0].message.tool_calls.as_ref().unwrap() {
                        let name = tool_call.function.name.clone().unwrap();
                        println!("Tool call: {name:?}");
                        let arguments = tool_call.function.arguments.clone().unwrap();
                        let response = match name.as_str() {
                            "artillery_fire" => Self::tool_artillery_fire(&arguments),
                            "artillery_available" => {
                                Self::tool_artillery_available(&inner, &arguments)
                            }
                            "artillery_eta" => Self::tool_artillery_eta(arguments).await,
                            _ => {
                                println!("Unknown tool call: {name:?}");
                                "unknown tool call".to_string()
                            }
                        };
                        inner.history.push(ChatCompletionMessage {
                            role: MessageRole::function,
                            content: Content::Text(response.clone()),
                            name: Some(name),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct CommanderInner {
    pub history: Vec<ChatCompletionMessage>,
    pub artillery: HashMap<String, Artillery>,
}

pub async fn speak(ctx: &Context, callsign: String, text: String) -> Result<(), String> {
    let id = Uuid::new_v4();
    let mut client = match openai_client(ctx) {
        Ok(client) => client,
        Err(err) => {
            eprintln!("Error creating OpenAI client: {err}");
            return Err("Error creating client".to_string());
        }
    };
    let req = AudioSpeechRequest::new(
        "gpt-4o-mini-tts".to_string(),
        text,
        "ash".to_string(),
        temp_dir()
            .join(format!("sai_{id}.mp3"))
            .display()
            .to_string(),
    )
    .instructions("Speak in a quick direct tone, you're a military support unit".to_string())
    .speed(1.4);
    let result = client.audio_speech(req).await;
    match result {
        Ok(result) if result.result => {
            println!("Audio file saved: {id}");
            if let Err(e) = ctx.callback_data("sai", "speak", (callsign, id)) {
                eprintln!("Error sending callback data: {e}");
                return Err("Error sending callback data".to_string());
            }
        }
        Ok(_) => {
            eprintln!("Error processing audio, result: {result:?}");
            return Err("Error processing audio file".to_string());
        }
        Err(err) => {
            eprintln!("Error processing audio: {err}");
            return Err("Error processing audio file".to_string());
        }
    }
    Ok(())
}

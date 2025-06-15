use std::collections::HashMap;

use arma_rs::{Context, ContextState, FromArma, Group, Value};
use openai_api_rs::v1::{
    chat_completion::{Tool, ToolType},
    types::{Function, FunctionParameters, JSONSchemaDefine, JSONSchemaType},
};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::{
    TokioContext,
    server::{commander::CommanderPool, response::ResponseManager},
};

use super::{Commander, CommanderInner};

#[derive(Debug, Serialize)]
pub struct Artillery {
    /// The Arma netid of the artillery unit
    pub id: String,
    /// The display name of the artillery unit
    pub name: String,
    /// The type of rounds available to the artillery unit
    pub rounds: Vec<Round>,
}

#[derive(Debug, Serialize)]
pub struct Round {
    /// The classname of the round
    pub classname: String,
    /// The display name of the round
    pub display_name: String,
    /// The quantity of rounds available
    pub quantity: u8,
}

impl FromArma for Round {
    fn from_arma(s: String) -> Result<Self, arma_rs::FromArmaError> {
        <(String, String, u8)>::from_arma(s).map(|(classname, display_name, quantity)| Self {
            classname,
            display_name,
            quantity,
        })
    }
}

impl Commander {
    pub async fn set_artillery(&self, id: String, name: String, rounds: Vec<Round>) {
        let artillery = Artillery {
            id: id.clone(),
            name,
            rounds,
        };
        self.inner.lock().await.artillery.insert(id, artillery);
        println!(
            "Artillery registered: {:?}",
            self.inner.lock().await.artillery
        );
    }

    pub fn tool_artillery_fire(arguments: &str) -> String {
        #[derive(Deserialize)]
        struct FireOrder {
            target: String,
            round: String,
            quantity: u8,
            unit: String,
            spread: Option<u32>,
        }
        println!("Tool fire called with arguments: {arguments:?}");
        let fire_order: FireOrder = serde_json::from_str(arguments).unwrap();
        if let Err(e) = TokioContext::get().unwrap().context.callback_data(
            "sai",
            "artillery:fire",
            (
                fire_order.target.replace([' ', ':', '-'], ""),
                fire_order.round,
                fire_order.quantity,
                fire_order.unit,
                fire_order.spread.unwrap_or(0),
            ),
        ) {
            eprintln!("Error sending callback data: {e}");
            return "Error requesting artillery fire".to_string();
        }
        println!("Callback data sent");
        "{\"success\": true}".to_string()
    }

    pub fn tool_artillery_available(inner: &CommanderInner, arguments: &str) -> String {
        println!("Tool available artillery called with arguments: {arguments:?}");
        let resp = serde_json::to_string(&inner.artillery).unwrap_or_else(|_| "[]".to_string());
        println!("Available artillery: {resp:?}");
        resp
    }

    pub async fn tool_artillery_eta(arguments: String) -> String {
        #[derive(Deserialize)]
        struct EtaOrder {
            target: String,
            round: String,
            unit: String,
        }
        println!("Tool artillery ETA called with arguments: {arguments:?}");
        let fire_order: EtaOrder = serde_json::from_str(&arguments).unwrap();
        let (id, mut recv) = ResponseManager::create();
        if let Err(e) = TokioContext::get().unwrap().context.callback_data(
            "sai",
            "artillery:eta",
            (
                id,
                fire_order.target.replace([' ', ':', '-'], ""),
                fire_order.round,
                fire_order.unit,
            ),
        ) {
            eprintln!("Error sending callback data: {e}");
            return "Error requesting artillery fire".to_string();
        }
        match recv.recv().await {
            Some(Value::Number(eta)) => {
                println!("Received ETA: {eta}");
                format!("{{\"eta\": {eta}}}")
            }
            Some(value) => {
                eprintln!("Received invalid response for artillery ETA: {value:?}");
                "{\"error\": \"Invalid response\"}".to_string()
            }
            None => {
                eprintln!("No response received for artillery ETA");
                "{\"error\": \"No response received\"}".to_string()
            }
        }
    }
}

pub fn group() -> Group {
    Group::new()
        .command("register", cmd_register)
        .command("remove", cmd_remove)
}

#[allow(clippy::needless_pass_by_value)]
fn cmd_register(ctx: Context, id: String, callsign: String, name: String, rounds: Vec<Round>) {
    println!("Registering artillery: {id} - {name}");
    println!("Rounds: {rounds:?}");
    ctx.global().get::<Runtime>().unwrap().block_on(async move {
        CommanderPool::get(&callsign)
            .set_artillery(id, name, rounds)
            .await;
    });
}

#[allow(clippy::needless_pass_by_value)]
fn cmd_remove(ctx: Context, id: String, callsign: String) {
    println!("Removing artillery: {id}");
    ctx.global().get::<Runtime>().unwrap().block_on(async move {
        let commander = CommanderPool::get(&callsign);
        commander.inner.lock().await.artillery.remove(&id);
        println!(
            "Artillery removed: {:?}",
            commander.inner.lock().await.artillery
        );
    });
}

pub fn tool_artillery_fire_schema() -> Tool {
    Tool {
        r#type: ToolType::Function,
        function: Function {
            name: "artillery_fire".to_string(),
            description: Some("Fire artillery at a position, requires user confirmation. Do not over-use, this is a destructive action.".to_string()),
            parameters: FunctionParameters {
                schema_type: JSONSchemaType::Object,
                properties:
                    Some(HashMap::from([
                        ("target".to_string(), Box::new(JSONSchemaDefine {
                            schema_type: Some(JSONSchemaType::String),
                            description: Some("The target to fire at, can be a grid reference in XY format. 6,8,10 digit grids are supported".to_string()),
                            ..Default::default()
                        })),
                        ("round".to_string(), Box::new(JSONSchemaDefine {
                            schema_type: Some(JSONSchemaType::String),
                            description: Some("The classname of round to fire, must be one of the available rounds for the artillery unit, can not contain spaces".to_string()),
                            ..Default::default()
                        })),
                        ("quantity".to_string(), Box::new(JSONSchemaDefine {
                            schema_type: Some(JSONSchemaType::Number),
                            description: Some("The number of rounds to fire".to_string()),
                            ..Default::default()
                        })),
                        ("unit".to_string(), Box::new(JSONSchemaDefine {
                            schema_type: Some(JSONSchemaType::String),
                            description: Some("The unit that should fire, must be in format x:y".to_string()),
                            ..Default::default()
                        })),
                        ("spread".to_string(), Box::new(JSONSchemaDefine {
                            schema_type: Some(JSONSchemaType::Number),
                            description: Some("The spread of the rounds, in meters".to_string()),
                            ..Default::default()
                        })),
                    ])),
                required: Some(vec![
                    "target".to_string(),
                    "round".to_string(),
                    "quantity".to_string(),
                    "unit".to_string(),
                ]),
            }
        }
    }
}

pub fn tool_artillery_available_schema() -> Tool {
    Tool {
        r#type: ToolType::Function,
        function: Function {
            name: "artillery_available".to_string(),
            description: Some(
                "Get a list of available artillery units, the id is private, do not tell users"
                    .to_string(),
            ),
            parameters: FunctionParameters {
                schema_type: JSONSchemaType::Object,
                properties: Some(HashMap::new()),
                required: None,
            },
        },
    }
}

pub fn tool_artillery_eta_schema() -> Tool {
    Tool {
        r#type: ToolType::Function,
        function: Function {
            name: "artillery_eta".to_string(),
            description: Some("Get the estimated time of arrival for artillery strikes. Returns -1 if the target can't be hit.".to_string()),
            parameters: FunctionParameters {
                schema_type: JSONSchemaType::Object,
                properties:
                    Some(HashMap::from([
                        ("target".to_string(), Box::new(JSONSchemaDefine {
                            schema_type: Some(JSONSchemaType::String),
                            description: Some("The target to fire at, can be a grid reference in XY format. 6,8,10 digit grids are supported".to_string()),
                            ..Default::default()
                        })),
                        ("round".to_string(), Box::new(JSONSchemaDefine {
                            schema_type: Some(JSONSchemaType::String),
                            description: Some("The classname of round to fire, must be one of the available rounds for the artillery unit, can not contain spaces".to_string()),
                            ..Default::default()
                        })),
                        ("unit".to_string(), Box::new(JSONSchemaDefine {
                            schema_type: Some(JSONSchemaType::String),
                            description: Some("The unit that should fire, must be in format x:y".to_string()),
                            ..Default::default()
                        })),
                    ])),
                required: Some(vec![
                    "target".to_string(),
                    "round".to_string(),
                    "unit".to_string(),
                ]),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use arma_rs::ContextState;
    use tokio::runtime::Runtime;

    use crate::server::commander::CommanderPool;

    #[test]
    pub fn artillery_fire() {
        let ext = crate::init().testing();
        let (ret, code) = ext.call_with_context(
            "server:commander:artillery:register",
            Some(vec![
                "\"1:1\"".to_string(),
                "\"hammer\"".to_string(),
                "\"Seara\"".to_string(),
                "[[\"he_177\",\"175mm HE\",12]]".to_string(),
            ]),
            arma_rs::Caller::Unknown,
            arma_rs::Source::Console,
            arma_rs::Mission::None,
            arma_rs::Server::Singleplayer,
            0,
        );
        assert_eq!(code, 0);
        println!("Code: {code:?}");
        println!("Ret: {ret:?}");
        let tokio_context = ext.context();
        ext.context()
            .global()
            .get::<Runtime>()
            .unwrap()
            .block_on(async {
                let commander = CommanderPool::get("test");
                let response = commander
                    .input(
                        &tokio_context,
                        "I need fire support at 1 4 8 2 0 4, send whatever you got".to_string(),
                    )
                    .await
                    .unwrap();
                println!("Response: {response:?}");
                let response = commander
                    .input(&tokio_context, "Confirm, fire 2 rounds".to_string())
                    .await
                    .unwrap();
                println!("Response: {response:?}");
            });
    }
}

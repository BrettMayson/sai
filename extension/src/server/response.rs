use std::sync::{Arc, LazyLock};

use arma_rs::{Context, ContextState as _, Value};
use dashmap::DashMap;
use tokio::{runtime::Runtime, sync::mpsc};
use uuid::Uuid;

pub struct ResponseManager {
    pub responses: Arc<DashMap<Uuid, mpsc::Sender<Value>>>,
}

impl ResponseManager {
    pub fn get() -> Self {
        static POOL: LazyLock<ResponseManager> = LazyLock::new(|| ResponseManager {
            responses: Arc::new(DashMap::new()),
        });
        Self {
            responses: POOL.responses.clone(),
        }
    }

    pub fn create() -> (Uuid, mpsc::Receiver<Value>) {
        let id = uuid::Uuid::new_v4();
        let (tx, rx) = mpsc::channel(1);
        Self::get().responses.insert(id, tx);
        (id, rx)
    }

    pub async fn send(id: &Uuid, data: Value) {
        let manager = Self::get();
        if let Some(sender) = manager.responses.get(id) {
            let _ = sender
                .send(data)
                .await
                .map_err(|_| "Failed to send response".to_string());
        } else {
            println!("No response channel found for ID: {id}");
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn cmd_response(ctx: Context, id: Uuid, data: Value) {
    ctx.global().get::<Runtime>().unwrap().spawn(async move {
        ResponseManager::send(&id, data).await;
    });
}

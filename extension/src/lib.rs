use std::sync::{Arc, OnceLock};

use arma_rs::{Context, ContextState, Extension, arma};
use tokio::runtime::Runtime;

mod client;
mod server;
mod settings;

#[arma]
fn init() -> Extension {
    let ext = Extension::build()
        .group("client", client::group())
        .group("server", server::group())
        .group("settings", settings::group())
        .finish();

    let ctx = ext.context();
    let ctx_tokio = ext.context();
    ctx.global().set::<Runtime>(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to initialize tokio runtime"),
    );

    TokioContext::init(ctx_tokio);

    ext
}

#[derive(Clone)]
pub struct TokioContext {
    context: Arc<Context>,
}
static TOKIO_CONTEXT_ONCE: OnceLock<TokioContext> = OnceLock::new();

impl TokioContext {
    pub fn get() -> Option<Self> {
        TOKIO_CONTEXT_ONCE.get().cloned()
    }

    pub fn init(ctx: Context) -> Self {
        TOKIO_CONTEXT_ONCE
            .get_or_init(|| Self {
                context: Arc::new(ctx),
            })
            .clone()
    }
}

impl std::ops::Deref for TokioContext {
    type Target = Arc<Context>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

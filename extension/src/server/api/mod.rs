use axum::{
    Router,
    routing::{get, post},
};

mod speak;
mod spoke;

pub fn app() -> Router {
    Router::new()
        .route("/spoke", post(spoke::handler))
        .route("/speak/{:id}", get(speak::handler))
}

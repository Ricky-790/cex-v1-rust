mod api;
mod engine;

use api::routes::{SharedEngine, cancel_order, get_orderbook, place_order};
use axum::{
    Router,
    routing::{delete, get, post},
};
use engine::Engine;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let engine: SharedEngine = Arc::new(Mutex::new(Engine::new()));

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/orderbook/:stock_id", get(get_orderbook))
        .route("/order", post(place_order))
        .route("/order", delete(cancel_order))
        .with_state(engine);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on port 3000");
    axum::serve(listener, app).await.unwrap();
}

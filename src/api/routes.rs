use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::engine::Engine;
use crate::engine::schemas::{OrderSide, OrderType};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// Declare sharedengine type
pub type SharedEngine = Arc<Mutex<Engine>>;

// Request Bodies
#[derive(Deserialize)]
pub struct PlaceOrderReq {
    pub user_id: u32,
    pub stock_id: u32,
    pub order_side: OrderSide,
    pub order_type: OrderType,
    pub quantity: f32,
    pub price: f32,
    pub amount: f32,
}
#[derive(Deserialize)]
pub struct CancelOrderReq {
    pub stock_id: u32,
    pub order_id: u32,
    pub order_side: OrderSide,
}

// Response schemas
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

// Handlers
pub async fn get_orderbook(
    Path(stock_id): Path<u32>,
    State(engine): State<SharedEngine>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = engine.lock().unwrap();
    match engine.order_books.get(&stock_id) {
        Some(orderbook) => Ok(Json(
            serde_json::json!({"success": true, "data": orderbook }),
        )),
        None => Err(StatusCode::NOT_FOUND),
    }
}
#[axum::debug_handler]
pub async fn place_order(
    State(engine): State<SharedEngine>,
    Json(body): Json<PlaceOrderReq>,
) -> Json<serde_json::Value> {
    let mut engine = engine.lock().unwrap();
    engine.add_order(
        body.user_id,
        body.order_type,
        body.order_side,
        body.stock_id,
        body.quantity,
        body.amount,
    );
    engine.match_order(body.stock_id);
    Json(serde_json::json!({
        "success": true,
        "message": "order placed and matched"
    }))
}
#[axum::debug_handler]
pub async fn cancel_order(
    State(engine): State<SharedEngine>,
    Json(body): Json<CancelOrderReq>,
) -> Json<serde_json::Value> {
    let mut engine = engine.lock().unwrap();
    engine.cancel_order(body.stock_id, body.order_id, body.order_side);
    Json(serde_json::json!({
        "success": true,
        "message": "order cancelled"
    }))
}

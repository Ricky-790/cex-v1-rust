use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum OrderSide {
    // Buy | Sell
    Buy,
    Sell,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum OrderType {
    // Limit | Market
    Market,
    Limit,
}
#[derive(Serialize, Deserialize)]
pub struct Balance {
    pub currency: String,
    pub locked: f32,
    pub available: f32,
}

pub struct StockCurrency {
    // pub stock_id: u32,
    pub base_currency: String,  // Ex: BTC
    pub quote_currency: String, // USD
}
#[derive(Serialize, Deserialize)]

pub struct Order {
    // One single order
    pub order_id: u32,
    pub user_id: u32,
    pub quantity: f32,
    pub price: f32,
    pub stock: u32,
    pub filled_quantity: f32,
    pub order_type: OrderType,
    pub order_side: OrderSide,
}
#[derive(Serialize, Deserialize)]

pub struct OrderBook {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub last_traded_price: Option<f32>,
}

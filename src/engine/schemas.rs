pub enum OrderSide {
    // Buy | Sell
    Buy,
    Sell,
}
pub enum OrderType {
    // Limit | Market
    Market,
    Limit,
}

pub struct Balance {
    pub currency: String,
    pub locked: f32,
    pub available: f32,
}

pub struct Order {
    // One single order
    pub order_id: u32,
    pub user_id: u32,
    pub quantity: f32,
    pub price: f32,
    pub stock: Option<u32>,
    pub filled_quantity: f32,
    pub order_type: OrderType,
    pub order_side: OrderSide,
}

pub struct OrderBook {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub last_traded_price: Option<f32>,
}

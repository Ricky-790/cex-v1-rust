mod schemas;
use crate::schemas::*;
use std::collections::HashMap;

struct Engine {
    order_number: u32,
    order_books: HashMap<u32, OrderBook>, // Stock-id : OrderBook
    users_balances: HashMap<u32, Vec<Balance>>, // User-id : All Balances
}

/*
Functions:
- Add order
- Match order (find best matches)
- fill order
- remove order
- cancel order

Helper Funcs:
- get balance
- Add bid
- Add ask
- Find user(in memory db)
- Find stock(in memory db)
*/

impl Engine {
    pub fn new() -> Engine {
        Engine {
            order_number: 0,
            order_books: HashMap::new(),
            users_balances: HashMap::new(),
        }
    }
    pub fn add_order(
        // Internally call add bid / add ask
        &mut self,
        user_id: u32,
        order_type: OrderType,
        order_side: OrderSide,
        stock_id: u32,
        quantity: f32,
        amount: f32,
    ) {
        /*
        1. get user
        2. get stock
        3. check & lock balance
        4. push order to orderbook
        */
    }
    pub fn match_order(&mut self) {}
    pub fn settle_trade(&mut self) {}
    pub fn remove_order(&mut self) {}
    pub fn cancel_order(&mut self) {}
    //Helper funcs
    fn get_balance(&self, currency: &str, user_id: u32) -> Option<&Balance> {
        let balances = self.users_balances.get(&user_id)?;
        balances.iter().find(|b| b.currency == currency)
    }
    fn get_balance_mut(&mut self, currency: &str, user_id: u32) -> Option<&mut Balance> {
        let balances = self.users_balances.get_mut(&user_id)?;
        balances.iter_mut().find(|b| b.currency == currency)
    }
    fn add_bid(&mut self, stock_id: u32, order: Order) {
        if let Some(orderbook) = self.order_books.get_mut(&stock_id) {
            let insert_position = orderbook.bids.partition_point(|b| b.price > order.price);
            orderbook.bids.insert(insert_position, order);
        }
    }
    fn add_ask(&mut self, stock_id: u32, order: Order) {
        if let Some(orderbook) = self.order_books.get_mut(&stock_id) {
            let insert_position = orderbook.asks.partition_point(|a| a.price < order.price);
            orderbook.asks.insert(insert_position, order);
        }
    }
    fn find_user(&mut self) {}
    fn find_stock(&mut self) {}
}

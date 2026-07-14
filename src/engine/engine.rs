mod schemas;
use crate::schemas::*;
use std::collections::HashMap;

struct Engine {
    order_number: u32,
    stocks: HashMap<u32, StockCurrency>,
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
            stocks: HashMap::new(),
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
        let (base_currency, quote_currency) = {
            let stock = self.stocks.get(&stock_id).unwrap();
            (stock.base_currency.clone(), stock.quote_currency.clone())
        }; // borrow of self.stocks ends here

        let currency = match order_side {
            OrderSide::Buy => &quote_currency,
            OrderSide::Sell => &base_currency,
        };
        //Check if there is sufficient balance

        //Get mutable balance, lock balance & add order using add_bid / add_ask
        if let Some(balance) = self.get_balance_mut(currency, user_id) {
            if balance.available >= quantity * amount {
                balance.available -= quantity * amount;
                balance.locked += quantity * amount;
                let new_order: Order = Order {
                    order_id: self.order_number,
                    user_id: user_id,
                    quantity: quantity,
                    price: amount,
                    stock: stock_id,
                    filled_quantity: 0.0,
                    order_type: order_type,
                    order_side: order_side,
                };
                if let OrderSide::Buy = order_side {
                    self.add_bid(stock_id, new_order)
                } else {
                    self.add_ask(stock_id, new_order);
                }
                self.order_number += 1;
            }
        }
    }
    pub fn match_order(&mut self) {}
    pub fn settle_trade(
        &mut self,
        price: f32,
        quantity: f32,
        stock_id: u32,
        bid_order_id: u32,
        ask_order_id: u32,
    ) {
        // Get market currencies
        let (base_currency, quote_currency) = {
            let stock = self.stocks.get(&stock_id).unwrap();
            (stock.base_currency.clone(), stock.quote_currency.clone())
        };

        // Get both user IDs before mutating balances
        let (bid_user_id, bid_qty) = {
            let bid = self
                .get_order_mut(OrderSide::Buy, bid_order_id, stock_id)
                .unwrap();
            bid.filled_quantity += quantity;
            (bid.user_id, bid.quantity)
        };
        let (ask_user_id, ask_qty) = {
            let ask = self
                .get_order_mut(OrderSide::Sell, ask_order_id, stock_id)
                .unwrap();
            ask.filled_quantity += quantity;
            (ask.user_id, ask.quantity)
        };

        // Update bid user balances (buyer: deduct locked USD, add base currency)
        if let Some(usd_bal) = self.get_balance_mut(&quote_currency, bid_user_id) {
            usd_bal.locked -= price * quantity;
        }
        if let Some(base_bal) = self.get_balance_mut(&base_currency, bid_user_id) {
            base_bal.available += quantity;
        } else {
            // create new currency balance
            if let Some(balances) = self.users_balances.get_mut(&bid_user_id) {
                balances.push(Balance {
                    currency: base_currency.clone(),
                    locked: 0.0,
                    available: quantity,
                });
            }
        }

        // Update ask user balances (seller: deduct locked base currency, add USD)
        if let Some(base_bal) = self.get_balance_mut(&base_currency, ask_user_id) {
            base_bal.locked -= quantity;
        }
        if let Some(usd_bal) = self.get_balance_mut(&quote_currency, ask_user_id) {
            usd_bal.available += price * quantity;
        } else {
            // seller doesn't have USD balance yet, create it
            if let Some(balances) = self.users_balances.get_mut(&ask_user_id) {
                balances.push(Balance {
                    currency: quote_currency.clone(),
                    locked: 0.0,
                    available: price * quantity,
                });
            }
        }

        // Update last traded price
        if let Some(orderbook) = self.order_books.get_mut(&stock_id) {
            orderbook.last_traded_price = Some(price);
        }

        // Remove fully filled orders
        if bid_qty == quantity {
            self.remove_order(stock_id, bid_order_id, OrderSide::Buy);
        }
        if ask_qty == quantity {
            self.remove_order(stock_id, ask_order_id, OrderSide::Sell);
        }
    }
    pub fn remove_order(&mut self, stock_id: u32, order_id: u32, order_side: OrderSide) {}
    pub fn cancel_order(&mut self) {}
    //Helper funcs
    fn get_order_mut(
        &mut self,
        order_side: OrderSide,
        order_id: u32,
        stock_id: u32,
    ) -> Option<&mut Order> {
        let orderbook = self.order_books.get_mut(&stock_id).unwrap();
        let orders = match order_side {
            OrderSide::Buy => &mut orderbook.bids,
            OrderSide::Sell => &mut orderbook.asks,
        };
        orders.iter_mut().find(|o| o.order_id == order_id)
    }

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
    // fn find_user(&mut self) {}
    // fn find_stock(&mut self) {}
}

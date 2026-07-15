use super::schemas::*;
use std::collections::HashMap;

pub struct Engine {
    pub order_number: u32,
    pub stocks: HashMap<u32, StockCurrency>,
    pub order_books: HashMap<u32, OrderBook>, // Stock-id : OrderBook
    pub users_balances: HashMap<u32, Vec<Balance>>, // User-id : All Balances
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
            let amount_to_lock = match order_side {
                OrderSide::Buy => quantity * amount, // lock USD (price * qty)
                OrderSide::Sell => quantity,         // lock BTC (just qty)
            };
            if balance.available >= amount_to_lock {
                balance.available -= amount_to_lock;
                balance.locked += amount_to_lock;

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
    pub fn match_order(&mut self, stock_id: u32) {
        loop {
            // Extract just the IDs and prices we need, then release the borrow
            let (bid_id, ask_id, ask_price, bid_qty, ask_qty) = {
                let orderbook = self.order_books.get(&stock_id).unwrap();

                match (orderbook.bids.first(), orderbook.asks.first()) {
                    (Some(bid), Some(ask)) if bid.price >= ask.price => {
                        // A match exists, extract what we need
                        (
                            bid.order_id,
                            ask.order_id,
                            ask.price,
                            bid.quantity,
                            ask.quantity,
                        )
                    }
                    // No match or one side is empty, stop looping
                    _ => break,
                }
            }; // immutable borrow of self ends here

            // Quantity traded is the smaller of the two orders
            let quantity = bid_qty.min(ask_qty);

            // Now we can mutably borrow self to settle
            self.settle_trade(ask_price, quantity, stock_id, bid_id, ask_id);
        }
    }

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
    pub fn remove_order(&mut self, stock_id: u32, order_id: u32, order_side: OrderSide) {
        let orderbook = self.order_books.get_mut(&stock_id).unwrap();
        let orders = match order_side {
            OrderSide::Buy => &mut orderbook.bids,
            OrderSide::Sell => &mut orderbook.asks,
        };
        orders.retain(|o| o.order_id != order_id)
    }
    pub fn cancel_order(&mut self, stock_id: u32, order_id: u32, order_side: OrderSide) {
        let (base_currency, quote_currency) = {
            let stock = self.stocks.get(&stock_id).unwrap();
            (stock.base_currency.clone(), stock.quote_currency.clone())
        };
        let (user_id, remaining_qty, price) = {
            let order = self.get_order_mut(order_side, order_id, stock_id).unwrap();
            let remaining = order.quantity - order.filled_quantity;
            (order.user_id, remaining, order.price)
        };

        //Release locked funds
        let currency_to_release = match order_side {
            OrderSide::Buy => &quote_currency, // release locked USD
            OrderSide::Sell => &base_currency, // release locked BTC/SOL etc
        };
        if let Some(balance) = self.get_balance_mut(currency_to_release, user_id) {
            balance.locked -= remaining_qty * price;
            balance.available += remaining_qty * price;
        }

        // 4. Remove the order
        self.remove_order(stock_id, order_id, order_side);
    }
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
/* AI Generated testsfor engine functionality */

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Helper to build a fresh engine with dummy data ----
    fn setup_engine() -> Engine {
        let mut engine = Engine::new();

        // Add BTC-USD market (stock_id: 1)
        engine.stocks.insert(
            1,
            StockCurrency {
                base_currency: String::from("BTC"),
                quote_currency: String::from("USD"),
            },
        );

        // Add SOL-USD market (stock_id: 2)
        engine.stocks.insert(
            2,
            StockCurrency {
                base_currency: String::from("SOL"),
                quote_currency: String::from("USD"),
            },
        );

        // Initialize empty orderbooks for both markets
        engine.order_books.insert(
            1,
            OrderBook {
                bids: vec![],
                asks: vec![],
                last_traded_price: None,
            },
        );
        engine.order_books.insert(
            2,
            OrderBook {
                bids: vec![],
                asks: vec![],
                last_traded_price: None,
            },
        );

        // User 1: has USD and BTC
        engine.users_balances.insert(
            1,
            vec![
                Balance {
                    currency: String::from("USD"),
                    locked: 0.0,
                    available: 10000.0,
                },
                Balance {
                    currency: String::from("BTC"),
                    locked: 0.0,
                    available: 2.0,
                },
            ],
        );

        // User 2: has USD and BTC
        engine.users_balances.insert(
            2,
            vec![
                Balance {
                    currency: String::from("USD"),
                    locked: 0.0,
                    available: 10000.0,
                },
                Balance {
                    currency: String::from("BTC"),
                    locked: 0.0,
                    available: 5.0,
                },
            ],
        );

        engine
    }

    // ---- Balance helpers ----
    fn get_available(engine: &Engine, user_id: u32, currency: &str) -> f32 {
        engine
            .users_balances
            .get(&user_id)
            .unwrap()
            .iter()
            .find(|b| b.currency == currency)
            .map(|b| b.available)
            .unwrap_or(0.0)
    }

    fn get_locked(engine: &Engine, user_id: u32, currency: &str) -> f32 {
        engine
            .users_balances
            .get(&user_id)
            .unwrap()
            .iter()
            .find(|b| b.currency == currency)
            .map(|b| b.locked)
            .unwrap_or(0.0)
    }

    // ================================================================
    // ADD ORDER TESTS
    // ================================================================

    #[test]
    fn test_add_buy_order_locks_usd() {
        let mut engine = setup_engine();
        // User 1 places a buy order: 1 BTC at $300
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 1.0, 300.0);

        assert_eq!(get_locked(&engine, 1, "USD"), 300.0);
        assert_eq!(get_available(&engine, 1, "USD"), 9700.0);
    }

    #[test]
    fn test_add_sell_order_locks_btc() {
        let mut engine = setup_engine();
        // User 1 places a sell order: 1 BTC at $300
        engine.add_order(1, OrderType::Limit, OrderSide::Sell, 1, 1.0, 300.0);

        // sells 1 BTC, so locks 1.0 BTC not 300.0
        assert_eq!(get_locked(&engine, 1, "BTC"), 1.0);
        assert_eq!(get_available(&engine, 1, "BTC"), 1.0); // had 2.0, sold 1.0
        // Note: you may want to add a minimum balance check to prevent negative available
    }

    #[test]
    fn test_add_buy_order_appears_in_bids() {
        let mut engine = setup_engine();
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 1.0, 300.0);

        let bids = &engine.order_books.get(&1).unwrap().bids;
        assert_eq!(bids.len(), 1);
        assert_eq!(bids[0].price, 300.0);
        assert_eq!(bids[0].quantity, 1.0);
        assert_eq!(bids[0].user_id, 1);
    }

    #[test]
    fn test_add_sell_order_appears_in_asks() {
        let mut engine = setup_engine();
        engine.add_order(2, OrderType::Limit, OrderSide::Sell, 1, 1.0, 300.0);

        let asks = &engine.order_books.get(&1).unwrap().asks;
        assert_eq!(asks.len(), 1);
        assert_eq!(asks[0].price, 300.0);
    }

    #[test]
    fn test_add_order_fails_if_insufficient_balance() {
        let mut engine = setup_engine();
        // User 1 tries to buy 100 BTC at $1000 each = $100,000, but only has $10,000
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 100.0, 1000.0);

        // Order should NOT be added
        let bids = &engine.order_books.get(&1).unwrap().bids;
        assert_eq!(bids.len(), 0);
        // Balance should be unchanged
        assert_eq!(get_available(&engine, 1, "USD"), 10000.0);
        assert_eq!(get_locked(&engine, 1, "USD"), 0.0);
    }

    #[test]
    fn test_bids_are_sorted_highest_price_first() {
        let mut engine = setup_engine();
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 1.0, 200.0);
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 1.0, 300.0);
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 1.0, 250.0);

        let bids = &engine.order_books.get(&1).unwrap().bids;
        assert_eq!(bids[0].price, 300.0);
        assert_eq!(bids[1].price, 250.0);
        assert_eq!(bids[2].price, 200.0);
    }

    #[test]
    fn test_asks_are_sorted_lowest_price_first() {
        let mut engine = setup_engine();
        engine.add_order(2, OrderType::Limit, OrderSide::Sell, 1, 1.0, 300.0);
        engine.add_order(2, OrderType::Limit, OrderSide::Sell, 1, 1.0, 200.0);
        engine.add_order(2, OrderType::Limit, OrderSide::Sell, 1, 1.0, 250.0);

        let asks = &engine.order_books.get(&1).unwrap().asks;
        assert_eq!(asks[0].price, 200.0);
        assert_eq!(asks[1].price, 250.0);
        assert_eq!(asks[2].price, 300.0);
    }

    // ================================================================
    // REMOVE ORDER TESTS
    // ================================================================

    #[test]
    fn test_remove_bid() {
        let mut engine = setup_engine();
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 1.0, 300.0);

        let order_id = engine.order_books.get(&1).unwrap().bids[0].order_id;
        engine.remove_order(1, order_id, OrderSide::Buy);

        let bids = &engine.order_books.get(&1).unwrap().bids;
        assert_eq!(bids.len(), 0);
    }

    #[test]
    fn test_remove_ask() {
        let mut engine = setup_engine();
        engine.add_order(2, OrderType::Limit, OrderSide::Sell, 1, 1.0, 300.0);

        let order_id = engine.order_books.get(&1).unwrap().asks[0].order_id;
        engine.remove_order(1, order_id, OrderSide::Sell);

        let asks = &engine.order_books.get(&1).unwrap().asks;
        assert_eq!(asks.len(), 0);
    }

    // ================================================================
    // CANCEL ORDER TESTS
    // ================================================================

    #[test]
    fn test_cancel_buy_order_releases_usd() {
        let mut engine = setup_engine();
        // Place order: lock $300
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 1.0, 300.0);
        assert_eq!(get_locked(&engine, 1, "USD"), 300.0);

        let order_id = engine.order_books.get(&1).unwrap().bids[0].order_id;
        engine.cancel_order(1, order_id, OrderSide::Buy);

        // Funds should be released
        assert_eq!(get_locked(&engine, 1, "USD"), 0.0);
        assert_eq!(get_available(&engine, 1, "USD"), 10000.0);
        // Order should be gone
        assert_eq!(engine.order_books.get(&1).unwrap().bids.len(), 0);
    }

    #[test]
    fn test_cancel_sell_order_releases_btc() {
        let mut engine = setup_engine();
        engine.add_order(2, OrderType::Limit, OrderSide::Sell, 1, 1.0, 300.0);

        let order_id = engine.order_books.get(&1).unwrap().asks[0].order_id;
        engine.cancel_order(1, order_id, OrderSide::Sell);

        assert_eq!(get_locked(&engine, 1, "BTC"), 0.0);
        assert_eq!(engine.order_books.get(&1).unwrap().asks.len(), 0);
    }

    // ================================================================
    // SETTLE TRADE TESTS
    // ================================================================

    #[test]
    fn test_settle_trade_full_fill() {
        let mut engine = setup_engine();

        // User 1 buys 1 BTC at $300
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 1.0, 300.0);
        // User 2 sells 1 BTC at $300
        engine.add_order(2, OrderType::Limit, OrderSide::Sell, 1, 1.0, 300.0);

        let bid_id = engine.order_books.get(&1).unwrap().bids[0].order_id;
        let ask_id = engine.order_books.get(&1).unwrap().asks[0].order_id;

        engine.settle_trade(300.0, 1.0, 1, bid_id, ask_id);

        // Buyer (user 1): USD locked reduced, BTC available increased
        assert_eq!(get_locked(&engine, 1, "USD"), 0.0);
        assert_eq!(get_available(&engine, 1, "BTC"), 3.0); // had 2, got 1

        // Seller (user 2): BTC locked reduced, USD available increased
        // Seller (user 2): BTC locked should be 0 after full fill
        assert_eq!(get_locked(&engine, 2, "BTC"), 0.0); // was 1.0 locked, now released
        assert_eq!(get_available(&engine, 2, "BTC"), 4.0); // had 5.0, sold 1.0
        assert_eq!(get_available(&engine, 2, "USD"), 10300.0); // had 10000, got 300

        // Both orders should be removed (fully filled)
        assert_eq!(engine.order_books.get(&1).unwrap().bids.len(), 0);
        assert_eq!(engine.order_books.get(&1).unwrap().asks.len(), 0);

        // Last traded price should be updated
        assert_eq!(
            engine.order_books.get(&1).unwrap().last_traded_price,
            Some(300.0)
        );
    }

    #[test]
    fn test_settle_trade_partial_fill() {
        let mut engine = setup_engine();

        // User 1 buys 2 BTC at $300
        engine.add_order(1, OrderType::Limit, OrderSide::Buy, 1, 2.0, 300.0);
        // User 2 sells 1 BTC at $300
        engine.add_order(2, OrderType::Limit, OrderSide::Sell, 1, 1.0, 300.0);

        let bid_id = engine.order_books.get(&1).unwrap().bids[0].order_id;
        let ask_id = engine.order_books.get(&1).unwrap().asks[0].order_id;

        // Only 1 BTC matched
        engine.settle_trade(300.0, 1.0, 1, bid_id, ask_id);

        // Ask is fully filled and removed, bid is partially filled and stays
        assert_eq!(engine.order_books.get(&1).unwrap().asks.len(), 0);
        assert_eq!(engine.order_books.get(&1).unwrap().bids.len(), 1);

        // Bid's filled_quantity should be updated
        assert_eq!(
            engine.order_books.get(&1).unwrap().bids[0].filled_quantity,
            1.0
        );
    }
}

```
cex-v1-rust/
├── Cargo.toml
└── src/
    ├── main.rs                  # Entry point, starts HTTP server + matching thread
    ├── engine/
    │   ├── mod.rs               # Exports engine
    │   ├── engine.rs            # Engine struct, singleton, init from DB/snapshot
    │   ├── matching.rs          # matchOrders loop
    │   ├── orders.rs            # addBid, addAsk, cancelOrder
    │   └── balances.rs          # lockFunds, releaseFunds, settleTrade
    ├── models/
    │   ├── mod.rs
    │   ├── order.rs             # Order, OrderBook structs
    │   ├── balance.rs           # Balance, UserBalances structs
    │   └── market.rs            # Market, Stock structs
    ├── api/
    │   ├── mod.rs
    │   ├── routes.rs            # Route definitions
    │   ├── auth.rs              # signin, signup handlers
    │   ├── orders.rs            # create_order, cancel_order, get_order handlers
    │   └── portfolio.rs         # get_fills, get_balances handlers
    ├── db/
    │   ├── mod.rs
    │   ├── client.rs            # DB connection setup
    │   ├── queries/
    │   │   ├── mod.rs
    │   │   ├── users.rs         # get_user, create_user
    │   │   ├── orders.rs        # get_order, get_open_orders
    │   │   └── fills.rs         # get_fills_by_user, create_fill
    │   └── migrations/          # SQL migration files
    ├── queue/
    │   ├── mod.rs
    │   └── worker.rs            # Queue consumer, DB write worker
    └── snapshot/
        ├── mod.rs
        ├── writer.rs            # Dump orderbook to JSON every 5 mins
        └── reader.rs            # Load snapshot on startup
```


Notes:
- `main.rs` is responsible for spinning up 3 things: the HTTP server, the matching thread, and the snapshot writer thread
- `engine.rs` holds the `Arc<Mutex<Engine>>` that gets shared across all three
- `queue/worker.rs` runs on its own thread too, consuming DB write tasks
- `snapshot/` is self contained — writer dumps, reader loads on startup, neither touches the matching logic directly
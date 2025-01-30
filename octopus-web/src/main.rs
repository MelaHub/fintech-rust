mod accounting;
mod core;

mod trading_platform;
use warp::Filter;

use std::sync::{Arc, Mutex};

use crate::trading_platform::TradingPlatform;
use octopus_common::errors::OctopusError;
use octopus_common::types::{
    AccountBalanceRequest, AccountUpdateRequest, Order, SendRequest,
};

async fn balance_request(
    account: AccountBalanceRequest,
    trading_platform: Arc<Mutex<TradingPlatform>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let account = account.signer;
    let mut ledger_lock = trading_platform.lock().unwrap();
    match ledger_lock.balance_of(&account) {
        Ok(balance) => Ok(warp::reply::json(&balance)),
        Err(e) => Err(warp::reject::custom(OctopusError(e))),
    }
}

async fn deposit(
    account: AccountUpdateRequest,
    trading_platform: Arc<Mutex<TradingPlatform>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut ledger_lock = trading_platform.lock().unwrap();
    match ledger_lock.deposit(&account.signer, account.amount) {
        Ok(tx) => Ok(warp::reply::json(&tx)),
        Err(e) => Err(warp::reject::custom(OctopusError(e))),
    }
}

async fn withdraw(
    account: AccountUpdateRequest,
    trading_platform: Arc<Mutex<TradingPlatform>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut ledger_lock = trading_platform.lock().unwrap();
    match ledger_lock.withdraw(&account.signer, account.amount) {
        Ok(tx) => Ok(warp::reply::json(&tx)),
        Err(e) => Err(warp::reject::custom(OctopusError(e))),
    }
}

async fn send(
    send_request: SendRequest,
    trading_platform: Arc<Mutex<TradingPlatform>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut ledger_lock = trading_platform.lock().unwrap();
    match ledger_lock.send(&send_request.from, &send_request.to, send_request.amount) {
        Ok(receipt) => Ok(warp::reply::json(&receipt)),
        Err(e) => Err(warp::reject::custom(OctopusError(e))),
    }
}

async fn order(
    order: Order,
    trading_platform: Arc<Mutex<TradingPlatform>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut ledger_lock = trading_platform.lock().unwrap();
    match ledger_lock.order(order) {
        Ok(receipt) => Ok(warp::reply::json(&receipt)),
        Err(e) => Err(warp::reject::custom(OctopusError(e))),
    }
}

async fn orderbook(
    trading_platform: Arc<Mutex<TradingPlatform>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let ledger_lock = trading_platform.lock().unwrap();
    Ok(warp::reply::json(&ledger_lock.orderbook()))
}

async fn transactions(
    trading_platform: Arc<Mutex<TradingPlatform>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let ledger_lock = trading_platform.lock().unwrap();
    Ok(warp::reply::json(&ledger_lock.transactions))
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let trading_platform = Arc::new(Mutex::new(TradingPlatform::new()));
    let trading_platform_state = warp::any().map(move || trading_platform.clone());

    let post_account = warp::path!("account")
        .and(warp::post())
        .and(warp::body::json())
        .and(trading_platform_state.clone())
        .and_then(balance_request);

    let post_deposit = warp::path!("account" / "deposit")
        .and(warp::post())
        .and(warp::body::json())
        .and(trading_platform_state.clone())
        .and_then(deposit);

    let post_withdraw = warp::path!("account" / "withdraw")
        .and(warp::post())
        .and(warp::body::json())
        .and(trading_platform_state.clone())
        .and_then(withdraw);

    let post_send = warp::path!("account" / "send")
        .and(warp::post())
        .and(warp::body::json())
        .and(trading_platform_state.clone())
        .and_then(send);

    let post_ordet = warp::path!("order")
        .and(warp::post())
        .and(warp::body::json())
        .and(trading_platform_state.clone())
        .and_then(order);

    let get_orderbook = warp::path!("orderbook")
        .and(warp::get())
        .and(trading_platform_state.clone())
        .and_then(orderbook);

    let get_transactions = warp::path!("txlog")
        .and(warp::get())
        .and(trading_platform_state.clone())
        .and_then(transactions);

    // Combine routes
    let routes = post_account
        .or(post_deposit)
        .or(post_withdraw)
        .or(post_send)
        .or(post_ordet)
        .or(get_orderbook)
        .or(get_transactions)
        .with(warp::cors().allow_any_origin());

    println!("Server running on http://localhost:3000");
    warp::serve(routes).run(([0, 0, 0, 0], 3000)).await;
}

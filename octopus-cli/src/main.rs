use std::{io, num::ParseIntError};

use reqwest;
use serde::{Deserialize, Serialize};
use tokio;

use clap::Parser;
use octopus_common::tx::Tx;
use octopus_common::types::{
    AccountBalanceRequest, AccountUpdateRequest, Order, PartialOrder, SendRequest, Side,
};

#[derive(Parser, Debug)]
struct Args {
    url: String,
}

fn read_order_parameters() -> Result<Order, String> {
    let account = read_from_stdin("Account:");
    let side = match read_from_stdin("Buy or Sell?:").to_lowercase().as_ref() {
        "buy" => Ok(Side::Buy),
        "sell" => Ok(Side::Sell),
        _ => Err("Unsupported order side"),
    }?;

    let amount = read_from_stdin("Amount:")
        .parse()
        .map_err(|e: ParseIntError| e.to_string())?;
    let price = read_from_stdin("Price:")
        .parse()
        .map_err(|e: ParseIntError| e.to_string())?;
    Ok(Order {
        price,
        amount,
        side,
        signer: account,
    })
}

fn read_from_stdin(label: &str) -> String {
    let mut buffer = String::new();
    println!("{}", label);
    io::stdin()
        .read_line(&mut buffer)
        .expect("Couldn't read from stdin");
    buffer.trim().to_owned()
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args = Args::parse();

    let url = args.url;

    println!(
        "Hello, accounting world! You'll send your requests to: {}",
        url
    );

    let client = reqwest::Client::new();

    loop {
        let input = read_from_stdin(
            "Choose operation [deposit, withdraw, send, print, txlog, order, orderbook, quit], confirm with return:",
        );
        match input.as_str() {
            "deposit" => {
                let account = read_from_stdin("Account:");

                let raw_amount = read_from_stdin("Amount:").parse();
                if let Ok(amount) = raw_amount {
                    let request = AccountUpdateRequest {
                        signer: account.clone(),
                        amount,
                    };
                    let deposit_url = format!("{}/account/deposit", url);
                    let response = client.post(deposit_url).json(&request).send().await?;

                    if !response.status().is_success() {
                        eprintln!("Something went wrong: {:?}", response);
                    }

                    println!("Deposited {} into account '{}'", amount, account)
                } else {
                    eprintln!("Not a number: '{:?}'", raw_amount);
                }
            }
            "withdraw" => {
                let account = read_from_stdin("Account:");
                let raw_amount = read_from_stdin("Amount:").parse();
                if let Ok(amount) = raw_amount {
                    let request = AccountUpdateRequest {
                        signer: account.clone(),
                        amount,
                    };
                    let withdraw_url = format!("{}/account/withdraw", url);
                    let response = client.post(withdraw_url).json(&request).send().await?;

                    if !response.status().is_success() {
                        eprintln!("Something went wrong: {:?}", response);
                    }
                    println!("Withdrawed {} from account '{}'", amount, account)
                } else {
                    eprintln!("Not a number: '{:?}'", raw_amount);
                }
            }
            "send" => {
                let sender = read_from_stdin("Sender Account:");
                let recipient = read_from_stdin("Recipient Account:");
                let raw_amount = read_from_stdin("Amount:").parse();
                if let Ok(amount) = raw_amount {
                    let request = SendRequest {
                        from: sender.clone(),
                        to: recipient.clone(),
                        amount,
                    };
                    let send_url = format!("{}/account/send", url);
                    let response = client.post(send_url).json(&request).send().await?;

                    if !response.status().is_success() {
                        eprintln!("Something went wrong: {:?}", response);
                    }
                    println!(
                        "Sent {} from account '{}' to '{}'",
                        amount, sender, recipient
                    )
                } else {
                    eprintln!("Not a number: '{:?}'", raw_amount);
                }
            }
            "order" => match read_order_parameters() {
                Ok(order) => {
                    let order_url = format!("{}/order", url);
                    let response = client.post(order_url).json(&order).send().await?;

                    if !response.status().is_success() {
                        eprintln!("Something went wrong: {:?}", response);
                    }
                    println!("Ordered: {:#?}", order);
                }
                Err(msg) => {
                    eprintln!("Invalid Order parameters: '{:?}'", msg);
                }
            },
            "orderbook" => {
                let orderbook_url = format!("{}/orderbook", url);
                let response = client.get(orderbook_url).send().await?;

                if !response.status().is_success() {
                    eprintln!("Something went wrong: {:?}", response);
                }
                let orderbook = response.json::<Vec<PartialOrder>>().await?;
                println!("The orderbook: {:#?}", orderbook);
            }
            "txlog" => {
                let txlog_url = format!("{}/txlog", url);
                let response = client.get(txlog_url).send().await?;

                if !response.status().is_success() {
                    eprintln!("Something went wrong: {:?}", response);
                }
                let transactions = response.json::<Vec<Tx>>().await?;
                println!("The TX log: {:#?}", transactions);
            }
            "print" => {
                let account = read_from_stdin("Account:");
                let request = AccountBalanceRequest {
                    signer: account.clone(),
                };
                let deposit_url = format!("{}/account", url);
                let response = client.post(deposit_url).json(&request).send().await?;

                if !response.status().is_success() {
                    eprintln!("Something went wrong: {:?}", response);
                }
                let balance = response.text().await?;
                println!("Account {} has balance '{}'", account, balance)
            }
            "quit" => {
                println!("Quitting...");
                break;
            }
            _ => {
                eprintln!("Invalid option: '{}'", input);
            }
        }
    }
    Ok(())
}

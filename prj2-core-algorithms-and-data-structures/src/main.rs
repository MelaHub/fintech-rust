use std::io;
mod accounting;
mod core;
mod errors;
mod trading_platform;
mod tx;

fn read_from_stdin(label: &str) -> String {
    let mut buffer = String::new();
    println!("{}", label);
    io::stdin()
        .read_line(&mut buffer)
        .expect("Couldn't read from stdin");
    buffer.trim().to_owned()
}

fn main() {
    println!("Hello, accounting world!");

    let mut trading_platform = trading_platform::TradingPlatform::new();
    let mut txlog = Vec::new();
    loop {
        let input = read_from_stdin(
            "Choose operation [deposit, withdraw, send, print, order, orderbook, txlog, quit], confirm with return:",
        );
        match input.as_str() {
            "deposit" => {
                let account = read_from_stdin("Account:");

                let raw_amount = read_from_stdin("Amount:").parse();
                if let Ok(amount) = raw_amount {
                    txlog.push(trading_platform.deposit(&account, amount).unwrap());
                    println!("Deposited {} into account '{}'", amount, account)
                } else {
                    eprintln!("Not a number: '{:?}'", raw_amount);
                }
            }
            "withdraw" => {
                let account = read_from_stdin("Account:");
                let raw_amount = read_from_stdin("Amount:").parse();
                if let Ok(amount) = raw_amount {
                    txlog.push(trading_platform.withdraw(&account, amount).unwrap());
                } else {
                    eprintln!("Not a number: '{:?}'", raw_amount);
                }
            }
            "send" => {
                let sender = read_from_stdin("Sender Account:");
                let recipient = read_from_stdin("Recipient Account:");
                let raw_amount = read_from_stdin("Amount:").parse();
                if let Ok(amount) = raw_amount {
                    let tx = trading_platform.send(&sender, &recipient, amount).unwrap();
                    txlog.push(tx.0);
                    txlog.push(tx.1);
                } else {
                    eprintln!("Not a number: '{:?}'", raw_amount);
                }
            }
            "print" => {
                println!("The trading_platform: {:?}", trading_platform);
            }
            "quit" => {
                println!("Quitting...");
                break;
            }
            "order" => {
                let signer = read_from_stdin("Account:");
                let side = read_from_stdin("Buy or Sell? [buy, sell]:");
                let amount = read_from_stdin("Amount:").parse().unwrap();
                let price = read_from_stdin("Price:").parse().unwrap();
                if side != "buy" && side != "sell" {
                    eprintln!("Invalid side: '{}'", side);
                    break;
                }
                
                let order = core::Order {
                    price,
                    amount,
                    side: if side == "buy" {
                        core::Side::Buy
                    } else {
                        core::Side::Sell
                    },
                    signer,
                };
                let _ = trading_platform.order(order);
            } 
            "orderbook" => {
                println!("Orderbook: {:?}", trading_platform.orderbook());
            }
            "txlog" => {
                println!("The txlog: {:?}", txlog);
            }
            _ => {
                eprintln!("Invalid option: '{}'", input);
            }
        }
    }
}

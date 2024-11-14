use std::collections::HashMap;
use std::num::ParseIntError;


/// An application-specific error type
#[derive(Debug)]
enum AccountingError {
    AccountOverFunded(String, u64),
    AccountUnderFunded(String, u64),
    AccountNotFound(String),
}

/// A transaction type. Transactions should be able to rebuild a ledger's state
/// when they are applied in the same sequence to an empty state.
#[derive(Debug)]
pub enum Tx {
    Deposit { account: String, amount: u64 },
    Withdraw { account: String, amount: u64 },
}

/// A type for managing accounts and their current currency balance
#[derive(Debug)]
struct Accounts {
    
    accounts: HashMap<String, u64>,
    
}

impl Accounts {

    /// Returns an empty instance of the [`Accounts`] type
    pub fn new() -> Self {
        Accounts {
            accounts: Default::default()
        }
    }

    /// Either deposits the `amount` provided into the `signer` account or adds the amount to the existing account.
    /// # Errors
    /// Attempted overflow
    pub fn deposit(&mut self, signer: &str, amount: u64) -> Result<Tx, AccountingError> {
        if let Some(account) = self.accounts.get_mut(signer) {
            (*account)
                .checked_add(amount)
                .and_then(|r| {
                    *account = r;
                    Some(r)
                })
                .ok_or(AccountingError::AccountOverFunded(
                    signer.to_string(),
                    amount,
                ))
                // Using map() here is an easy way to only manipulate the non-error result
                .map(|_| Tx::Deposit {
                    account: signer.to_string(),
                    amount,
                })
        } else {
            self.accounts.insert(signer.to_string(), amount);
            Ok(Tx::Deposit {
                account: signer.to_string(),
                amount,
            })
        }
    }

    /// Withdraws the `amount` from the `signer` account.
    /// # Errors
    /// Attempted overflow
    pub fn withdraw(&mut self, signer: &str, amount: u64) -> Result<Tx, AccountingError> {
        match self.accounts.get_mut(signer) {
            Some(account) => {
                account
                    .checked_sub(amount)
                    .and_then(|r| {
                        *account = r;
                        Some(r)
                    })
                    .ok_or(AccountingError::AccountUnderFunded(signer.to_string(), amount))?;
                Ok(Tx::Withdraw { account: signer.to_string(), amount })
            }
            None => Err(AccountingError::AccountNotFound(signer.to_string()))
        }
    }

    /// Withdraws the amount from the sender account and deposits it in the recipient account.
    ///
    /// # Errors
    /// The account doesn't exist
    pub fn send(
        &mut self,
        sender: &str,
        recipient: &str,
        amount: u64,
    ) -> Result<(Tx, Tx), AccountingError> {

        if let Some(_) = self.accounts.get(recipient) {
            let tx1 = self.withdraw(sender, amount)?;
            let tx2 = self.deposit(recipient, amount)?;
            Ok((tx1, tx2))
        } else {
            return Err(AccountingError::AccountNotFound(recipient.to_string()));
        }
    }
    
}

fn read_from_stdin(label: &str) -> String {
    use std::io::{self, Write};
    print!("{}", label);
    io::stdout().flush().unwrap();
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim().to_string()
}

fn read_int_from_stdin(label: &str) -> Result<u64, ParseIntError> {
    read_from_stdin(label)
        .parse::<u64>()
}

fn main() {
    let mut ledger = Accounts::new();
    loop {
        let command = read_from_stdin("Enter a command: ");
        match command.as_str() {
            "deposit" => {
                let account = read_from_stdin("Enter account: ");
                let amount = read_int_from_stdin("Enter amount: ");
                match amount {
                    Ok(amount) => {
                        let status = ledger.deposit(&account, amount);
                        println!("Depositing {} for {}: {:?}", account, amount, status);
                    },
                    Err(e) => {
                        println!("Please provide a positive integer value: {:?}", e);
                    }
                }
            },
            "withdraw" => {
                let account = read_from_stdin("Enter account: ");
                let amount = read_int_from_stdin("Enter amount: ");
                match amount {
                    Ok(amount) => {
                        let status = ledger.withdraw(&account, amount);
                        println!("Withdrawing {} for {}: {:?}", account, amount, status);
                    },
                    Err(e) => {
                        println!("Please provide a positive integer value: {:?}", e);
                    }
                }
            },
            "send" => {
                let sender = read_from_stdin("Enter sender: ");
                let recipient = read_from_stdin("Enter recipient: ");
                let amount = read_int_from_stdin("Enter amount: ");
                match amount {
                    Ok(amount) => {
                        let status = ledger.send(&sender, &recipient, amount);
                        println!("Sent {} from {} to {}: {:?}", amount, sender, recipient, status);
                    },
                    Err(e) => {
                        println!("Please provide a positive integer value: {:?}", e);
                    }
                }
            },
            "print" => {
                println!("Ledger: {:?}", ledger);
            },
            "quit" => {
                break;
            },
            _ => {
                println!("Command '{}' not found", command);
            }
        };
    }

    /* 
    println!("Hello, accounting world!");

    // We are using simple &str instances as keys
    // for more sophisticated keys (e.g. hashes)
    // the data type could remain the same
    let bob = "bob";
    let alice = "alice";
    let charlie = "charlie";
    let initial_amount = 100;

    // Creates the basic ledger and a tx log container
    let mut ledger = Accounts::new();
    let mut tx_log = vec![];

    // Deposit an amount to each account
    for signer in &[bob, alice, charlie] {
        let status = ledger.deposit(*signer, initial_amount);
        println!("Depositing {} for {}: {:?}", signer, initial_amount, status);
        // Add the resulting transaction to a list of transactions
        // .unwrap() will crash the program if the status is an error.
        tx_log.push(status.unwrap());
    }

    // Send currency from one account (bob) to the other (alice)
    let send_amount = 10_u64;
    let status = ledger.send(bob, alice, send_amount);
    println!(
        "Sent {} from {} to {}: {:?}",
        send_amount, bob, alice, status
    );

    // Add both transactions to the transaction log
    let (tx1, tx2) = status.unwrap();
    tx_log.push(tx1);
    tx_log.push(tx2);

    // Withdraw everything from the accounts
    let tx = ledger.withdraw(charlie, initial_amount).unwrap();
    tx_log.push(tx);
    let tx = ledger
        .withdraw(alice, initial_amount + send_amount)
        .unwrap();
    tx_log.push(tx);

    // Here we are withdrawing too much and there won't be a transaction
    println!(
        "Withdrawing {} from {}: {:?}",
        initial_amount,
        bob,
        ledger.withdraw(bob, initial_amount)
    );
    // Withdrawing the expected amount results in a transaction
    let tx = ledger.withdraw(bob, initial_amount - send_amount).unwrap();
    tx_log.push(tx);

    // {:?} prints the Debug implementation, {:#?} pretty-prints it
    println!("Ledger empty: {:?}", ledger);
    println!("The TX log: {:#?}", tx_log);
    */
}

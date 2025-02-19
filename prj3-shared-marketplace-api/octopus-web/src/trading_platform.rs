use octopus_common::{
    errors::ApplicationError,
    tx::Tx,
    types::{Order, PartialOrder, Receipt, Side},
};

use crate::{accounting::Accounts, core::MatchingEngine};

/// The core of the core: the [`TradingPlatform`]. Manages accounts, validates-, and orchestrates the processing of each order.
///
///
pub struct TradingPlatform {
    pub matching_engine: MatchingEngine,
    pub accounts: Accounts,
    pub transactions: Vec<Tx>,
}

impl TradingPlatform {
    /// Creates a new instance without any data.
    pub fn new() -> Self {
        TradingPlatform {
            matching_engine: MatchingEngine::new(),
            accounts: Accounts::new(),
            transactions: vec![],
        }
    }

    /// Fetches the complete order book at this time
    pub fn orderbook(&self) -> Vec<PartialOrder> {
        self.matching_engine
            .asks
            .values()
            .cloned()
            .into_iter()
            .chain(self.matching_engine.bids.values().cloned().into_iter())
            .flatten()
            .collect()
    }

    /// Withdraw funds
    pub fn balance_of(&mut self, signer: &str) -> Result<&u64, ApplicationError> {
        self.accounts.balance_of(signer)
    }

    /// Deposit funds
    pub fn deposit(&mut self, signer: &str, amount: u64) -> Result<Tx, ApplicationError> {
        self.accounts.deposit(signer, amount).map(|tx| {
            self.transactions.push(tx.clone());
            tx
        })
    }

    /// Withdraw funds
    pub fn withdraw(&mut self, signer: &str, amount: u64) -> Result<Tx, ApplicationError> {
        self.accounts.withdraw(signer, amount).map(|tx| {
            self.transactions.push(tx.clone());
            tx
        })
    }

    /// Transfer funds between sender and recipient
    pub fn send(
        &mut self,
        sender: &str,
        recipient: &str,
        amount: u64,
    ) -> Result<(Tx, Tx), ApplicationError> {
        self.accounts.send(sender, recipient, amount).map(|tx| {
            self.transactions.push(tx.0.clone());
            self.transactions.push(tx.1.clone());
            tx
        })
    }

    /// Process a given order and apply the outcome to the accounts involved. Note that there are very few safeguards in place.
    ///
    /// # Errors
    /// - Account has insufficient funds
    pub fn order(&mut self, order: Order) -> Result<Receipt, ApplicationError> {
        let total_amount = order.amount * order.price;
        // Make sure the account has a deposit
        match self.balance_of(&order.signer) {
            Ok(balance) if &order.side == &Side::Buy && balance < &total_amount => {
                return Err(ApplicationError::AccountUnderFunded(
                    order.signer.clone(),
                    total_amount,
                ))
            }
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let signer = order.signer.clone();
        let side = order.side.clone();
        // Do the actual matching
        let receipt = self.matching_engine.process(order)?;

        receipt
            .matches
            .iter()
            .map(|m| match side {
                Side::Buy => self.send(&signer, &m.signer, m.amount * m.price),
                Side::Sell => self.send(&m.signer, &signer, m.amount * m.price),
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    // reduce the warnings for naming tests
    #![allow(non_snake_case)]

    use super::*;

    #[test]
    fn test_TradingPlatform_order_requires_deposit_to_order() {
        let mut trading_platform = TradingPlatform::new();

        assert_eq!(
            trading_platform.order(Order {
                price: 10,
                amount: 1,
                side: Side::Sell,
                signer: "ALICE".to_string(),
            }),
            Err(ApplicationError::AccountNotFound("ALICE".to_string()))
        );
        assert!(trading_platform.matching_engine.asks.is_empty());
        assert!(trading_platform.matching_engine.bids.is_empty());
    }

    #[test]
    fn test_TradingPlatform_order_partially_match_order_updates_accounts() {
        let mut trading_platform = TradingPlatform::new();

        // Set up accounts
        assert!(trading_platform.accounts.deposit("ALICE", 100).is_ok());
        assert!(trading_platform.accounts.deposit("BOB", 100).is_ok());

        let alice_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 1,
                side: Side::Sell,
                signer: "ALICE".to_string(),
            })
            .unwrap();
        assert_eq!(alice_receipt.matches, vec![]);
        assert_eq!(alice_receipt.ordinal, 1);

        let bob_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 2,
                side: Side::Buy,
                signer: "BOB".to_string(),
            })
            .unwrap();

        assert_eq!(
            bob_receipt.matches,
            vec![PartialOrder {
                price: 10,
                amount: 1,
                remaining: 0,
                side: Side::Sell,
                signer: "ALICE".to_string(),
                ordinal: 1
            }]
        );
        assert!(trading_platform.matching_engine.asks.is_empty());
        assert_eq!(trading_platform.matching_engine.bids.len(), 1);

        // Check the account balances
        assert_eq!(trading_platform.accounts.balance_of("ALICE"), Ok(&110));
        assert_eq!(trading_platform.accounts.balance_of("BOB"), Ok(&90));
    }

    #[test]
    fn test_TradingPlatform_order_fully_match_order_updates_accounts() {
        let mut trading_platform = TradingPlatform::new();

        // Set up accounts
        assert!(trading_platform.accounts.deposit("ALICE", 100).is_ok());
        assert!(trading_platform.accounts.deposit("BOB", 100).is_ok());

        let alice_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 2,
                side: Side::Sell,
                signer: "ALICE".to_string(),
            })
            .unwrap();
        assert_eq!(alice_receipt.matches, vec![]);
        assert_eq!(alice_receipt.ordinal, 1);

        let bob_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 2,
                side: Side::Buy,
                signer: "BOB".to_string(),
            })
            .unwrap();

        assert_eq!(
            bob_receipt.matches,
            vec![PartialOrder {
                price: 10,
                amount: 2,
                remaining: 0,
                side: Side::Sell,
                signer: "ALICE".to_string(),
                ordinal: 1
            }]
        );

        // A fully matched order doesn't remain in the book
        assert!(trading_platform.matching_engine.asks.is_empty());
        assert!(trading_platform.matching_engine.bids.is_empty());

        // Check the account balances
        assert_eq!(trading_platform.accounts.balance_of("ALICE"), Ok(&120));
        assert_eq!(trading_platform.accounts.balance_of("BOB"), Ok(&80));
    }

    #[test]
    fn test_TradingPlatform_order_fully_match_order_multi_match_updates_accounts() {
        let mut trading_platform = TradingPlatform::new();

        // Set up accounts
        assert!(trading_platform.accounts.deposit("ALICE", 100).is_ok());
        assert!(trading_platform.accounts.deposit("BOB", 100).is_ok());
        assert!(trading_platform.accounts.deposit("CHARLIE", 100).is_ok());

        let alice_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 1,
                side: Side::Sell,
                signer: "ALICE".to_string(),
            })
            .unwrap();
        assert_eq!(alice_receipt.matches, vec![]);
        assert_eq!(alice_receipt.ordinal, 1);

        let charlie_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 1,
                side: Side::Sell,
                signer: "CHARLIE".to_string(),
            })
            .unwrap();
        assert_eq!(charlie_receipt.matches, vec![]);
        assert_eq!(charlie_receipt.ordinal, 2);

        let bob_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 2,
                side: Side::Buy,
                signer: "BOB".to_string(),
            })
            .unwrap();

        assert_eq!(
            bob_receipt.matches,
            vec![
                PartialOrder {
                    price: 10,
                    amount: 1,
                    remaining: 0,
                    side: Side::Sell,
                    signer: "ALICE".to_string(),
                    ordinal: 1
                },
                PartialOrder {
                    price: 10,
                    amount: 1,
                    remaining: 0,
                    side: Side::Sell,
                    signer: "CHARLIE".to_string(),
                    ordinal: 2
                }
            ]
        );
        // A fully matched order doesn't remain in the book
        assert!(trading_platform.matching_engine.asks.is_empty());
        assert!(trading_platform.matching_engine.bids.is_empty());

        // Check account balances
        assert_eq!(trading_platform.accounts.balance_of("ALICE"), Ok(&110));
        assert_eq!(trading_platform.accounts.balance_of("BOB"), Ok(&80));
        assert_eq!(trading_platform.accounts.balance_of("CHARLIE"), Ok(&110));
    }

    #[test]
    fn test_TradingPlatform_order_fully_match_order_no_self_match_updates_accounts() {
        let mut trading_platform = TradingPlatform::new();

        // Set up accounts
        assert!(trading_platform.accounts.deposit("ALICE", 100).is_ok());
        assert!(trading_platform.accounts.deposit("CHARLIE", 100).is_ok());

        let alice_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 1,
                side: Side::Sell,
                signer: "ALICE".to_string(),
            })
            .unwrap();
        assert_eq!(alice_receipt.matches, vec![]);
        assert_eq!(alice_receipt.ordinal, 1);

        let charlie_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 1,
                side: Side::Sell,
                signer: "CHARLIE".to_string(),
            })
            .unwrap();
        assert_eq!(charlie_receipt.matches, vec![]);
        assert_eq!(charlie_receipt.ordinal, 2);

        let bob_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 2,
                side: Side::Buy,
                signer: "ALICE".to_string(),
            })
            .unwrap();

        assert_eq!(
            bob_receipt.matches,
            vec![PartialOrder {
                price: 10,
                amount: 1,
                remaining: 0,
                side: Side::Sell,
                signer: "CHARLIE".to_string(),
                ordinal: 2
            }]
        );
        // A fully matched order doesn't remain in the book
        assert_eq!(trading_platform.matching_engine.asks.len(), 1);
        assert_eq!(trading_platform.matching_engine.bids.len(), 1);
        // Check account balances
        assert_eq!(trading_platform.accounts.balance_of("ALICE"), Ok(&90));
        assert_eq!(trading_platform.accounts.balance_of("CHARLIE"), Ok(&110));
    }

    #[test]
    fn test_TradingPlatform_order_no_match_updates_accounts() {
        let mut trading_platform = TradingPlatform::new();

        // Set up accounts
        assert!(trading_platform.accounts.deposit("ALICE", 100).is_ok());
        assert!(trading_platform.accounts.deposit("BOB", 100).is_ok());

        let alice_receipt = trading_platform
            .order(Order {
                price: 10,
                amount: 2,
                side: Side::Sell,
                signer: "ALICE".to_string(),
            })
            .unwrap();
        assert_eq!(alice_receipt.matches, vec![]);
        assert_eq!(alice_receipt.ordinal, 1);

        let bob_receipt = trading_platform
            .order(Order {
                price: 11,
                amount: 2,
                side: Side::Sell,
                signer: "BOB".to_string(),
            })
            .unwrap();

        assert_eq!(bob_receipt.matches, vec![]);
        assert_eq!(trading_platform.orderbook().len(), 2);

        // Check the account balances
        assert_eq!(trading_platform.accounts.balance_of("ALICE"), Ok(&100));
        assert_eq!(trading_platform.accounts.balance_of("BOB"), Ok(&100));
    }
}

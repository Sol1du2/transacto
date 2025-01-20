use anyhow::Result;
use enum_dispatch::enum_dispatch;
use thiserror::Error;

pub mod client;
pub mod ledger;
pub mod transactions;

use transactions::{Chargeback, Deposit, Dispute, Resolve, Transaction, Withdrawal};

#[derive(Debug, PartialEq, Error)]
pub enum TransactionError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("account is locked")]
    AccountLocked,
    #[error("invalid amount")]
    InvalidAmount,
    #[error("client not found")]
    ClientNotFound,
    #[error("transaction not found")]
    TransactionNotFound,
    #[error("dispute not supported for this transaction")]
    DisputeNotSupported,
    #[error("transaction is under a dispute")]
    TransactionUnderDispute,
    #[error("transaction already has a resolved dispute")]
    TransactionAlreadyDisputed,
    #[error("transaction is not under a dispute")]
    TransactionNotDisputed,
}

#[enum_dispatch]
pub trait ExecutableTransaction {
    fn execute(&self, ledger: &mut ledger::Ledger) -> Result<(), TransactionError>;

    fn dispute(&mut self, client: &mut client::Client) -> Result<(), TransactionError>;
    fn resolve(&mut self, client: &mut client::Client) -> Result<(), TransactionError>;
    fn chargeback(&mut self, client: &mut client::Client) -> Result<(), TransactionError>;

    fn id(&self) -> Option<u32>;
}

use std::fs::File;

use anyhow::Result;
use log::debug;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::accounting::client::Client;
use crate::accounting::ledger::Ledger;
use crate::accounting::{
    transactions::{Chargeback, Deposit, Dispute, Resolve, Transaction, Withdrawal},
    TransactionError,
};

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Error)]
pub enum TransactionDataError {
    #[error("transaction requires amount")]
    MissingAmount,
    #[error("{0}")]
    TransactionCreationError(#[from] TransactionError),
}

#[derive(Debug, Deserialize)]
pub struct TransactionRecord {
    #[serde(rename = "tx")]
    pub id: u32,
    #[serde(rename = "type")]
    pub type_: TransactionType,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(default)] // Default to `None` if the field is empty
    pub amount: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct ClientRecord {
    #[serde(rename = "client")]
    pub id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl From<&Client> for ClientRecord {
    fn from(client: &Client) -> Self {
        ClientRecord {
            id: client.id(),
            available: client.available(),
            held: client.held(),
            total: client.get_total(),
            locked: client.locked(),
        }
    }
}

impl TryFrom<TransactionRecord> for Transaction {
    type Error = TransactionDataError;

    fn try_from(tx: TransactionRecord) -> Result<Self, Self::Error> {
        match tx.type_ {
            TransactionType::Deposit => {
                if let Some(amount) = tx.amount {
                    Ok(Transaction::Deposit(Deposit::new(tx.id, tx.client_id, amount)?))
                } else {
                    Err(TransactionDataError::MissingAmount)
                }
            },

            TransactionType::Withdrawal => {
                if let Some(amount) = tx.amount {
                    Ok(Transaction::Withdrawal(Withdrawal::new(tx.id, tx.client_id, amount)?))
                } else {
                    Err(TransactionDataError::MissingAmount)
                }
            },
            TransactionType::Dispute => Ok(Transaction::Dispute(Dispute::new(tx.id, tx.client_id))),
            TransactionType::Resolve => Ok(Transaction::Resolve(Resolve::new(tx.id, tx.client_id))),
            TransactionType::Chargeback => Ok(Transaction::Chargeback(Chargeback::new(tx.id, tx.client_id))),
        }
    }
}

pub fn process_csv(file_path: &str, ledger: &mut Ledger) -> Result<()> {
    let file = File::open(file_path)?;
    let mut csv_reader = csv::ReaderBuilder::new().trim(csv::Trim::All).from_reader(file);

    for record in csv_reader.deserialize::<TransactionRecord>() {
        match record {
            Ok(transaction) => match transaction.try_into() {
                Ok(transaction) => {
                    if let Err(err) = ledger.execute_transaction(transaction) {
                        debug!("failed to execute transaction, err={}", err);
                    }
                },
                Err(err) => debug!("invalid transaction, err={}", err),
            },
            Err(err) => debug!("failed to deserialize record, err={}", err),
        }
    }

    Ok(())
}

pub fn export_csv(ledger: &Ledger) -> Result<()> {
    let mut csv_writer = csv::WriterBuilder::new().from_writer(std::io::stdout());
    for (_id, client) in ledger.clients_iter() {
        let record: ClientRecord = client.into();
        csv_writer.serialize(record)?;
    }

    csv_writer.flush()?;

    Ok(())
}

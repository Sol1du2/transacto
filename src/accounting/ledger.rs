use std::collections::hash_map::Iter;
use std::collections::HashMap;

use super::client::Client;
use super::transactions::Transaction;
use super::{ExecutableTransaction, TransactionError};

#[derive(Default)]
pub struct Ledger {
    pub clients: HashMap<u16, Client>,
    pub transactions: HashMap<u32, Transaction>,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            clients: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    /// Transactions that have their own global unique id will be stored.
    /// If the id already exists then the transaction is discarded.
    pub fn execute_transaction(&mut self, transaction: Transaction) -> Result<(), TransactionError> {
        if let Some(id) = transaction.id() {
            if self.transactions.contains_key(&id) {
                // The transaction has already been processed, ignore.
                return Ok(());
            }
        }

        transaction.execute(self)?;

        // Transactions that contain their own id could potentially be reversed,
        // so we should store them.
        if let Some(id) = transaction.id() {
            self.transactions.insert(id, transaction);
        }

        Ok(())
    }

    pub fn clients_iter(&self) -> Iter<u16, Client> {
        self.clients.iter()
    }
}

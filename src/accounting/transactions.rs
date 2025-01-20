use anyhow::Result;
use enum_dispatch::enum_dispatch;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::{client, ExecutableTransaction, TransactionError};
use super::{client::Client, ledger::Ledger};

#[enum_dispatch(ExecutableTransaction)]
pub enum Transaction {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, PartialEq)]
pub enum DisputeStatus {
    NoDispute,
    InDispute,
    Resolved,
    Chargedback,
}

impl DisputeStatus {
    fn under_dispute(&self) -> bool {
        self == &DisputeStatus::InDispute
    }

    fn dispute_solved(&self) -> bool {
        self == &DisputeStatus::Resolved || self == &DisputeStatus::Chargedback
    }
}

pub struct Deposit {
    id: u32,
    client_id: u16,
    amount: Decimal,

    dispute_status: DisputeStatus,
}

impl Deposit {
    pub fn new(id: u32, client_id: u16, amount: Decimal) -> Result<Deposit, TransactionError> {
        if amount <= dec!(0) {
            return Err(TransactionError::InvalidAmount);
        }

        Ok(Deposit {
            id,
            client_id,
            amount,
            dispute_status: DisputeStatus::NoDispute,
        })
    }
}

impl ExecutableTransaction for Deposit {
    fn execute(&self, ledger: &mut Ledger) -> Result<(), TransactionError> {
        let client = ledger
            .clients
            .entry(self.client_id)
            .or_insert(Client::new(self.client_id));

        client.deposit(self.amount);

        Ok(())
    }

    fn dispute(&mut self, client: &mut Client) -> Result<(), TransactionError> {
        if self.dispute_status.under_dispute() {
            return Err(TransactionError::TransactionUnderDispute);
        }

        if self.dispute_status.dispute_solved() {
            return Err(TransactionError::TransactionAlreadyDisputed);
        }

        client.hold_funds(self.amount);
        self.dispute_status = DisputeStatus::InDispute;

        Ok(())
    }

    fn resolve(&mut self, client: &mut client::Client) -> Result<(), TransactionError> {
        if self.dispute_status.dispute_solved() {
            return Err(TransactionError::TransactionAlreadyDisputed);
        }

        if !self.dispute_status.under_dispute() {
            return Err(TransactionError::TransactionNotDisputed);
        }

        client.release_funds(self.amount);
        self.dispute_status = DisputeStatus::Resolved;

        Ok(())
    }

    fn chargeback(&mut self, client: &mut Client) -> Result<(), TransactionError> {
        if self.dispute_status.dispute_solved() {
            return Err(TransactionError::TransactionAlreadyDisputed);
        }

        if !self.dispute_status.under_dispute() {
            return Err(TransactionError::TransactionNotDisputed);
        }

        client.chargeback(self.amount);
        self.dispute_status = DisputeStatus::Chargedback;

        Ok(())
    }

    fn id(&self) -> Option<u32> {
        Some(self.id)
    }
}

pub struct Withdrawal {
    id: u32,
    client_id: u16,
    amount: Decimal,
}

impl Withdrawal {
    pub fn new(id: u32, client_id: u16, amount: Decimal) -> Result<Withdrawal, TransactionError> {
        if amount <= dec!(0) {
            return Err(TransactionError::InvalidAmount);
        }

        Ok(Withdrawal { id, client_id, amount })
    }
}

impl ExecutableTransaction for Withdrawal {
    fn execute(&self, ledger: &mut Ledger) -> Result<(), TransactionError> {
        if let Some(client) = ledger.clients.get_mut(&self.client_id) {
            client.withdraw(self.amount)
        } else {
            Err(TransactionError::ClientNotFound)
        }
    }

    fn dispute(&mut self, _client: &mut Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn resolve(&mut self, _client: &mut client::Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn chargeback(&mut self, _client: &mut Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn id(&self) -> Option<u32> {
        Some(self.id)
    }
}

pub struct Dispute {
    ref_tx_id: u32,
    client_id: u16,
}

impl Dispute {
    pub fn new(ref_tx_id: u32, client_id: u16) -> Dispute {
        Dispute { ref_tx_id, client_id }
    }
}

impl ExecutableTransaction for Dispute {
    fn execute(&self, ledger: &mut Ledger) -> Result<(), TransactionError> {
        if let Some(client) = ledger.clients.get_mut(&self.client_id) {
            if let Some(transaction) = ledger.transactions.get_mut(&self.ref_tx_id) {
                transaction.dispute(client)
            } else {
                Err(TransactionError::TransactionNotFound)
            }
        } else {
            Err(TransactionError::ClientNotFound)
        }
    }

    fn dispute(&mut self, _client: &mut Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn resolve(&mut self, _client: &mut client::Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn chargeback(&mut self, _client: &mut Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn id(&self) -> Option<u32> {
        None
    }
}

pub struct Resolve {
    ref_tx_id: u32,
    client_id: u16,
}

impl Resolve {
    pub fn new(ref_tx_id: u32, client_id: u16) -> Resolve {
        Resolve { ref_tx_id, client_id }
    }
}

impl ExecutableTransaction for Resolve {
    fn execute(&self, ledger: &mut Ledger) -> Result<(), TransactionError> {
        if let Some(client) = ledger.clients.get_mut(&self.client_id) {
            if let Some(transaction) = ledger.transactions.get_mut(&self.ref_tx_id) {
                transaction.resolve(client)
            } else {
                Err(TransactionError::TransactionNotFound)
            }
        } else {
            Err(TransactionError::ClientNotFound)
        }
    }

    fn dispute(&mut self, _client: &mut Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn resolve(&mut self, _client: &mut client::Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn chargeback(&mut self, _client: &mut Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn id(&self) -> Option<u32> {
        None
    }
}

pub struct Chargeback {
    ref_tx_id: u32,
    client_id: u16,
}

impl Chargeback {
    pub fn new(ref_tx_id: u32, client_id: u16) -> Chargeback {
        Chargeback { ref_tx_id, client_id }
    }
}

impl ExecutableTransaction for Chargeback {
    fn execute(&self, ledger: &mut Ledger) -> Result<(), TransactionError> {
        if let Some(client) = ledger.clients.get_mut(&self.client_id) {
            if let Some(transaction) = ledger.transactions.get_mut(&self.ref_tx_id) {
                transaction.chargeback(client)
            } else {
                Err(TransactionError::TransactionNotFound)
            }
        } else {
            Err(TransactionError::ClientNotFound)
        }
    }

    fn dispute(&mut self, _client: &mut Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn resolve(&mut self, _client: &mut client::Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn chargeback(&mut self, _client: &mut Client) -> Result<(), TransactionError> {
        Err(TransactionError::DisputeNotSupported)
    }

    fn id(&self) -> Option<u32> {
        None
    }
}

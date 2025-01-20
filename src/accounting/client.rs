use anyhow::Result;
use rust_decimal::Decimal;

use super::TransactionError;

const PRECISION: u32 = 4;

pub struct Client {
    pub id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Client {
    pub fn new(id: u16) -> Client {
        Client {
            id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        }
    }

    pub fn deposit(&mut self, amount: Decimal) {
        self.available = (self.available + amount).round_dp(PRECISION);
    }

    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), TransactionError> {
        if self.locked {
            return Err(TransactionError::AccountLocked);
        }

        if self.available < amount {
            return Err(TransactionError::InsufficientFunds);
        }

        self.available = (self.available - amount).round_dp(PRECISION);

        Ok(())
    }

    pub fn hold_funds(&mut self, amount: Decimal) {
        self.available = (self.available - amount).round_dp(PRECISION);
        self.held = (self.held + amount).round_dp(PRECISION);
    }

    pub fn release_funds(&mut self, amount: Decimal) {
        self.held = (self.held - amount).round_dp(PRECISION);
        self.available = (self.available + amount).round_dp(PRECISION);
    }

    pub fn chargeback(&mut self, amount: Decimal) {
        self.held = (self.held - amount).round_dp(PRECISION);
        self.locked = true;
    }

    pub fn get_total(&self) -> Decimal {
        self.available + self.held
    }
}

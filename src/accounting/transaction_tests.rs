use anyhow::{bail, Result};
use pretty_assertions::assert_eq;
use rust_decimal_macros::dec;

use super::*;

fn assert_client(client: &Client, id: u16, available: Decimal, held: Decimal, locked: bool) {
    assert_eq!(client.id, id);
    assert_eq!(client.available, available);
    assert_eq!(client.held, held);
    assert_eq!(client.locked, locked);
    assert_eq!(client.get_total(), available + held);
}

#[test]
fn test_deposit() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(10))?))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(10), dec!(0), false);

    Ok(())
}

#[test]
fn test_deposit_keeps_4_decimal() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(3.1415926535))?))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(3.1416), dec!(0), false);

    Ok(())
}

#[test]
fn test_withdrawal() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(10))?))?;
    ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(1, 0, dec!(7))?))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(3), dec!(0), false);

    Ok(())
}

#[test]
fn test_withdrawal_client_not_found() -> Result<()> {
    let mut ledger = Ledger::new();
    if let Err(err) = ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(0, 0, dec!(7))?)) {
        assert_eq!(err, TransactionError::ClientNotFound);
    } else {
        bail!("withdrawal should not create a new client");
    }

    assert_eq!(ledger.clients.is_empty(), true);

    Ok(())
}

#[test]
fn test_withdrawal_insufficient_funds() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(10))?))?;

    if let Err(err) = ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(1, 0, dec!(10.0001))?)) {
        assert_eq!(err, TransactionError::InsufficientFunds);
    } else {
        bail!("withdrawal should not work if client has insufficient funds");
    }

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(10), dec!(0), false);

    Ok(())
}

#[test]
fn test_withdrawal_locked_account() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(51))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(1, 0, dec!(15))?))?;
    ledger.execute_transaction(Transaction::Dispute(Dispute::new(0, 0)))?;
    ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(0, 0)))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(15), dec!(0), true);

    if let Err(err) = ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(2, 0, dec!(10.55))?)) {
        assert_eq!(err, TransactionError::AccountLocked);
    } else {
        bail!("withdrawal should not work if client has insufficient funds");
    }

    Ok(())
}

#[test]
fn test_dispute() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(1, 0, dec!(5.9))?))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(10.9), dec!(0), false);

    ledger.execute_transaction(Transaction::Dispute(Dispute::new(1, 0)))?;
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(5), dec!(5.9), false);

    Ok(())
}

#[test]
fn test_dispute_client_not_found() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    if let Err(err) = ledger.execute_transaction(Transaction::Dispute(Dispute::new(0, 1))) {
        assert_eq!(err, TransactionError::ClientNotFound);
    } else {
        bail!("dispute should not create a new client");
    }

    assert_eq!(ledger.clients.len(), 1);

    Ok(())
}

#[test]
fn test_dispute_transaction_not_found() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    if let Err(err) = ledger.execute_transaction(Transaction::Dispute(Dispute::new(2, 0))) {
        assert_eq!(err, TransactionError::TransactionNotFound);
    } else {
        bail!("dispute should reference a known transaction");
    }

    Ok(())
}

#[test]
fn test_dispute_invalid_transaction() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(1, 0, dec!(2))?))?;

    if let Err(err) = ledger.execute_transaction(Transaction::Dispute(Dispute::new(1, 0))) {
        assert_eq!(err, TransactionError::DisputeNotSupported);
    } else {
        bail!("dispute should only be available to deposits");
    }

    Ok(())
}

#[test]
fn test_resolve() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(2, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(1, 1, dec!(55))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(51))?))?;
    ledger.execute_transaction(Transaction::Dispute(Dispute::new(0, 0)))?;

    assert_eq!(ledger.clients.len(), 2);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(5), dec!(51), false);

    ledger.execute_transaction(Transaction::Resolve(Resolve::new(0, 0)))?;

    assert_eq!(ledger.clients.len(), 2);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(56), dec!(0), false);

    Ok(())
}

#[test]
fn test_resolve_client_not_found() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    if let Err(err) = ledger.execute_transaction(Transaction::Resolve(Resolve::new(0, 1))) {
        assert_eq!(err, TransactionError::ClientNotFound);
    } else {
        bail!("resolve should not create a new client");
    }

    assert_eq!(ledger.clients.len(), 1);

    Ok(())
}

#[test]
fn test_resolve_transaction_not_found() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    if let Err(err) = ledger.execute_transaction(Transaction::Resolve(Resolve::new(2, 0))) {
        assert_eq!(err, TransactionError::TransactionNotFound);
    } else {
        bail!("resolve should reference a known transaction");
    }

    Ok(())
}

#[test]
fn test_resolve_invalid_transaction() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(1, 0, dec!(2))?))?;

    if let Err(err) = ledger.execute_transaction(Transaction::Resolve(Resolve::new(1, 0))) {
        assert_eq!(err, TransactionError::DisputeNotSupported);
    } else {
        bail!("resolve should only be available to deposits");
    }

    Ok(())
}

#[test]
fn test_resolve_not_in_dispute() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;

    if let Err(err) = ledger.execute_transaction(Transaction::Resolve(Resolve::new(0, 0))) {
        assert_eq!(err, TransactionError::TransactionNotDisputed);
    } else {
        bail!("resolve should only work on disputing transactions");
    }

    Ok(())
}

#[test]
fn test_chargeback() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(2, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(1, 1, dec!(55))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(51))?))?;
    ledger.execute_transaction(Transaction::Dispute(Dispute::new(0, 0)))?;

    assert_eq!(ledger.clients.len(), 2);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(5), dec!(51), false);

    ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(0, 0)))?;

    assert_eq!(ledger.clients.len(), 2);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(5), dec!(0), true);

    Ok(())
}

#[test]
fn test_chargeback_client_not_found() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    if let Err(err) = ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(0, 1))) {
        assert_eq!(err, TransactionError::ClientNotFound);
    } else {
        bail!("chargeback should not create a new client");
    }

    assert_eq!(ledger.clients.len(), 1);

    Ok(())
}

#[test]
fn test_chargeback_transaction_not_found() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    if let Err(err) = ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(2, 0))) {
        assert_eq!(err, TransactionError::TransactionNotFound);
    } else {
        bail!("chargeback should reference a known transaction");
    }

    Ok(())
}

#[test]
fn test_chargeback_invalid_transaction() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(1, 0, dec!(2))?))?;

    if let Err(err) = ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(1, 0))) {
        assert_eq!(err, TransactionError::DisputeNotSupported);
    } else {
        bail!("chargeback should only be available to deposits");
    }

    Ok(())
}

#[test]
fn test_chargeback_not_in_dispute() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;

    if let Err(err) = ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(0, 0))) {
        assert_eq!(err, TransactionError::TransactionNotDisputed);
    } else {
        bail!("chargeback should only work on disputing transactions");
    }

    Ok(())
}

#[test]
fn test_already_disputed() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(2, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(1, 1, dec!(55))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(51))?))?;
    ledger.execute_transaction(Transaction::Dispute(Dispute::new(0, 0)))?;
    ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(0, 0)))?;

    if let Err(err) = ledger.execute_transaction(Transaction::Resolve(Resolve::new(0, 0))) {
        assert_eq!(err, TransactionError::TransactionAlreadyDisputed);
    } else {
        bail!("transaction should not be disputed again");
    }

    if let Err(err) = ledger.execute_transaction(Transaction::Dispute(Dispute::new(0, 0))) {
        assert_eq!(err, TransactionError::TransactionAlreadyDisputed);
    } else {
        bail!("transaction should not be resolved again");
    }

    if let Err(err) = ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(0, 0))) {
        assert_eq!(err, TransactionError::TransactionAlreadyDisputed);
    } else {
        bail!("transaction should not be chargedback again");
    }

    assert_eq!(ledger.clients.len(), 2);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(5), dec!(0), true);

    Ok(())
}

#[test]
fn test_dispute_negative_funds() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(1, 0, dec!(3.2))?))?;
    ledger.execute_transaction(Transaction::Dispute(Dispute::new(0, 0)))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(-3.2), dec!(5), false);

    ledger.execute_transaction(Transaction::Chargeback(Chargeback::new(0, 0)))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(-3.2), dec!(0), true);

    Ok(())
}

#[test]
fn test_transaction_invalid_amount() -> Result<()> {
    if let Err(err) = Deposit::new(0, 0, dec!(-59)) {
        assert_eq!(err, TransactionError::InvalidAmount);
    } else {
        bail!("negative amounts should not be allowed");
    }

    if let Err(err) = Withdrawal::new(0, 0, dec!(-15.589)) {
        assert_eq!(err, TransactionError::InvalidAmount);
    } else {
        bail!("negative amounts should not be allowed");
    }

    if let Err(err) = Deposit::new(0, 0, dec!(0)) {
        assert_eq!(err, TransactionError::InvalidAmount);
    } else {
        bail!("negative amounts should not be allowed");
    }

    if let Err(err) = Withdrawal::new(0, 0, dec!(0)) {
        assert_eq!(err, TransactionError::InvalidAmount);
    } else {
        bail!("negative amounts should not be allowed");
    }

    Ok(())
}

#[test]
fn test_ignore_repeated_transaction() -> Result<()> {
    let mut ledger = Ledger::new();
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;
    ledger.execute_transaction(Transaction::Deposit(Deposit::new(0, 0, dec!(5))?))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(5), dec!(0), false);

    ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(1, 0, dec!(2))?))?;
    ledger.execute_transaction(Transaction::Withdrawal(Withdrawal::new(1, 0, dec!(2))?))?;

    assert_eq!(ledger.clients.len(), 1);
    assert_client(ledger.clients.get(&0).unwrap(), 0, dec!(3), dec!(0), false);

    Ok(())
}

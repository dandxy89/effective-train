use std::collections::BTreeMap;

use anyhow::{bail, Ok, Result};

use crate::{
    account::ClientState,
    data::{
        Transaction,
        TransactionType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal},
    },
};

pub trait Transact {
    fn chargeback(&mut self, tx: &Transaction, chargeback_tx: &Transaction) -> Result<()>;
    fn deposit(&mut self, tx: &Transaction) -> Result<()>;
    fn dispute(&mut self, tx: &Transaction, disputed_tx: &mut Transaction) -> Result<()>;
    fn resolve(&mut self, tx: &Transaction, disputed_tx: &mut Transaction) -> Result<()>;
    fn withdraw(&mut self, tx: &Transaction) -> Result<()>;
}

pub struct Ledger {
    accounts: BTreeMap<u16, ClientState>,
    approved_tx: BTreeMap<u32, Transaction>,
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            approved_tx: BTreeMap::new(),
        }
    }

    fn record_tx(&mut self, tx: &Transaction) -> Result<()> {
        self.approved_tx.insert(tx.tx_id(), tx.clone());
        Ok(())
    }

    fn process_transaction(&mut self, tx: &mut Transaction) -> Result<()> {
        let state = self
            .accounts
            .entry(tx.client_id())
            .or_insert_with(|| ClientState::new(tx.client_id()));

        match (tx.tx_type(), self.approved_tx.get_mut(&tx.tx_id())) {
            (Deposit, _) => state.deposit(tx).and_then(|_| self.record_tx(tx)),
            (Withdrawal, _) => state.withdraw(tx).and_then(|_| self.record_tx(tx)),
            (Dispute, Some(disputed_tx)) => state.dispute(tx, disputed_tx).map(|_| {
                disputed_tx.in_dispute = true;
                ()
            }),
            (Resolve, Some(disputed_tx)) => state.resolve(tx, disputed_tx).map(|_| {
                disputed_tx.in_dispute = false;
                ()
            }),
            (Chargeback, Some(chargeback_tx)) => state.chargeback(tx, chargeback_tx),
            _ => bail!("Unmatched transaction `{}`", tx.tx_id()),
        }
    }
}

#[cfg(test)]
mod test {
    use rust_decimal::{prelude::FromPrimitive, Decimal};

    use crate::{
        data::{Transaction, TransactionType},
        ledger::Ledger,
    };

    #[test]
    fn load_and_record_transaction() {
        let mut test_ledger = Ledger::new();
        let mut deposit_tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(200.).unwrap()),
            in_dispute: false,
        };
        let mut withdrawal_tx = Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: 123,
            tx_id: 2,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };
        let mut tx = Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 123,
            tx_id: 2,
            amount: None,
            in_dispute: false,
        };

        test_ledger.process_transaction(&mut deposit_tx).unwrap();
        test_ledger.process_transaction(&mut withdrawal_tx).unwrap();

        assert_eq!(test_ledger.accounts.len(), 1);
        assert_eq!(test_ledger.approved_tx.len(), 2);

        let user_account = test_ledger.accounts.get(&123).unwrap();
        assert_eq!(user_account.available().to_string(), "100");
        assert_eq!(user_account.held().to_string(), "0");
        assert_eq!(user_account.total().to_string(), "100");

        test_ledger.process_transaction(&mut tx).unwrap();
        assert_eq!(test_ledger.approved_tx.len(), 2);
        dbg!(&test_ledger.approved_tx);
        let disputed_tx = test_ledger.approved_tx.get(&2).unwrap();
        dbg!(&disputed_tx);
        assert!(disputed_tx.in_dispute());

        let disputed_tx = test_ledger.approved_tx.get(&2).unwrap();
        dbg!(&disputed_tx);
        assert!(disputed_tx.in_dispute());

        let mut resolve_tx = Transaction {
            tx_type: TransactionType::Resolve,
            client_id: 123,
            tx_id: 2,
            amount: None,
            in_dispute: false,
        };
        test_ledger.process_transaction(&mut resolve_tx).unwrap();
        let disputed_tx = test_ledger.approved_tx.get(&2).unwrap();
        dbg!(&disputed_tx);
        assert!(!disputed_tx.in_dispute());
    }
}

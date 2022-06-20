#![allow(clippy::module_name_repetitions)]
use anyhow::{bail, Ok, Result};
use rust_decimal::Decimal;

use crate::{data::Transaction, ledger::Transact};

/// A client account with valid transactions
pub struct ClientState {
    client_id: u16,
    available: Decimal,
    held: Decimal,
    /// An account is locked if a chargeback occurs
    locked: bool,
}

impl ClientState {
    pub fn new(client_id: u16) -> Self {
        Self {
            client_id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        }
    }

    pub fn id(&self) -> u16 {
        self.client_id
    }

    pub fn available(&self) -> Decimal {
        self.available
    }

    pub fn held(&self) -> Decimal {
        self.held
    }

    pub fn total(&self) -> Decimal {
        self.available.saturating_add(self.held)
    }

    fn account_ready(&self, client_id: u16) -> Result<()> {
        if self.locked {
            bail!("Account '{}' is locked", self.client_id)
        } else if client_id != self.client_id {
            bail!(
                "Client Id mismatch between Transaction Client Id and Client Id '{}'",
                client_id
            )
        }

        Ok(())
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

impl Transact for ClientState {
    fn chargeback(&mut self, tx: &Transaction, chargeback_tx: &Transaction) -> Result<()> {
        self.account_ready(tx.client_id())?;
        self.locked = true;

        match chargeback_tx.amount() {
            Some(amount) => {
                self.held = self.held.saturating_sub(amount);
                Ok(())
            }
            _ => bail!("Chargeback to Client account '{}' failed", self.client_id),
        }
    }

    fn deposit(&mut self, tx: &Transaction) -> Result<()> {
        self.account_ready(tx.client_id())?;

        match tx.amount() {
            Some(amount) => {
                self.available = self.available.saturating_add(amount);
                Ok(())
            }
            _ => bail!("Deposit to Client account '{}' failed", self.client_id),
        }
    }

    fn dispute(&mut self, tx: &Transaction, disputed_tx: &mut Transaction) -> Result<()> {
        self.account_ready(tx.client_id())?;
        self.account_ready(disputed_tx.client_id())?;

        match disputed_tx.amount() {
            Some(amount) if disputed_tx.is_disputable() => {
                self.available = self.available.saturating_sub(amount);
                self.held = self.held.saturating_add(amount);
                disputed_tx.dispute();
                Ok(())
            }
            _ => {
                bail!("Transaction `{}` cannot be disputed", disputed_tx.tx_id())
            }
        }
    }

    fn resolve(&mut self, tx: &Transaction, disputed_tx: &mut Transaction) -> Result<()> {
        self.account_ready(tx.client_id())?;
        self.account_ready(disputed_tx.client_id())?;

        match disputed_tx.amount() {
            Some(amount) if disputed_tx.in_dispute() => {
                self.available = self.available.saturating_add(amount);
                self.held = self.held.saturating_sub(amount);
                disputed_tx.in_dispute = false;
                Ok(())
            }
            _ if !disputed_tx.in_dispute() => {
                bail!(
                    "Resolving Transaction failed as TxId `{}` is not under dispute",
                    disputed_tx.tx_id()
                )
            }
            _ => bail!("Attempting to resolve a dispute but has not got a amount"),
        }
    }

    fn withdraw(&mut self, tx: &Transaction) -> Result<()> {
        self.account_ready(tx.client_id())?;

        match tx.amount() {
            Some(amount) if self.available >= amount => {
                self.available = self.available.saturating_sub(amount);
                Ok(())
            }
            Some(amount) if self.available < amount => {
                bail!(
                    "Withdrawal failed due to insufficient funds in Client Account `{}`",
                    self.client_id
                )
            }
            _ => bail!("Withdrawal to Client account '{}' failed", self.client_id),
        }
    }
}

#[cfg(test)]
mod test {
    use rust_decimal::{prelude::FromPrimitive, Decimal};

    use crate::{
        account::ClientState,
        data::{Transaction, TransactionType},
        ledger::Transact,
    };

    #[test]
    fn validate_account_totals() {
        let mut ac = ClientState::new(1);
        assert_eq!(ac.available().to_string(), "0");
        assert_eq!(ac.held().to_string(), "0");
        assert_eq!(ac.total().to_string(), "0");
        ac.available += Decimal::new(100, 0);
        assert_eq!(ac.total().to_string(), ac.available().to_string());

        ac.held += Decimal::new(10, 0);
        assert_eq!(
            (ac.total() - ac.held()).to_string(),
            ac.available().to_string()
        );
    }

    #[test]
    fn deposit_into_unlocked_account() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        };
        let tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };

        // Should SUCCEED: When the account is unlocked it should succeed
        let result = user_account.deposit(&tx);
        assert!(result.is_ok());
    }

    #[test]
    fn deposit_should_fail_when_account_is_locked() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        };
        let tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };

        user_account.locked = true;
        let result = user_account.deposit(&tx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Account '123' is locked".to_string()
        );
    }

    #[test]
    fn deposit_should_failed_when_ids_conflicts() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        };
        let mut tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };

        // Should FAIL: When the account client id is different from the tx id
        user_account.locked = false;
        tx.client_id = 2;
        let result = user_account.deposit(&tx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Client Id mismatch between Transaction Client Id and Client Id '2'".to_string()
        );
    }

    #[test]
    fn withdrawal_should_succeed_when_unlocked_and_sufficient_balance() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let tx = Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };

        // Should SUCCEED: When the account is unlocked it should succeed
        let result = user_account.withdraw(&tx);
        assert!(result.is_ok());
    }

    #[test]
    fn withdrawal_should_succeed_when_locked() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let tx = Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };

        // Should FAIL: When the account is locked it should fail
        user_account.locked = true;
        let result = user_account.withdraw(&tx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Account '123' is locked".to_string()
        );
    }

    #[test]
    fn withdrawal_should_fail_when_client_ids_are_mismatched() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let mut tx = Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };

        // Should FAIL: When the account client id is different from the tx id
        user_account.locked = false;
        tx.client_id = 2;
        let result = user_account.withdraw(&tx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Client Id mismatch between Transaction Client Id and Client Id '2'".to_string()
        );
    }

    #[test]
    fn withdrawal_should_fail_when_client_id_has_insufficient_funds() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let tx = Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(120.).unwrap()),
            in_dispute: false,
        };

        // Should FAIL: When available funds < tx.amount
        let result = user_account.withdraw(&tx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Withdrawal failed due to insufficient funds in Client Account `123`".to_string()
        );
    }

    #[test]
    fn basic_dispute_actions() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let mut disputed_tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };
        let tx = Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 123,
            tx_id: 1,
            amount: None,
            in_dispute: false,
        };
        let result = user_account.deposit(&disputed_tx);
        assert!(result.is_ok());

        // Should SUCCEED: To move amounts to held and set
        let result = user_account.dispute(&tx, &mut disputed_tx);
        assert!(result.is_ok());
        assert!(disputed_tx.in_dispute());
        assert!(user_account.held() == disputed_tx.amount.unwrap());
    }

    #[test]
    fn attempting_dispute_on_mismatched_client_ids() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let disputed_tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };
        let tx = Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 123,
            tx_id: 1,
            amount: None,
            in_dispute: false,
        };
        let result = user_account.deposit(&disputed_tx);
        assert!(result.is_ok());

        // Should SUCCEED: To generate an error when tx.client_id != disputed.client_id
        let mut disputed_tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1234,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };
        let result = user_account.dispute(&tx, &mut disputed_tx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Client Id mismatch between Transaction Client Id and Client Id '1234'".to_string()
        );
    }

    #[test]
    fn disputed_transaction_has_no_amount() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let disputed_tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };
        let tx = Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 123,
            tx_id: 1,
            amount: None,
            in_dispute: false,
        };
        let result = user_account.deposit(&disputed_tx);
        assert!(result.is_ok());

        // Should FAIL: To do dispute if the disputed transaction as no amount
        let mut disputed_tx = Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 123,
            tx_id: 1,
            amount: None,
            in_dispute: false,
        };
        let result = user_account.dispute(&tx, &mut disputed_tx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Transaction `1` cannot be disputed".to_string()
        );
    }

    #[test]
    fn basic_resolve_actions() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let mut disputed_tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };
        let dispute_tx = Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 123,
            tx_id: 1,
            amount: None,
            in_dispute: false,
        };
        let resolve_tx = Transaction {
            tx_type: TransactionType::Resolve,
            client_id: 123,
            tx_id: 1,
            amount: None,
            in_dispute: false,
        };

        user_account.deposit(&disputed_tx).unwrap();
        user_account.dispute(&dispute_tx, &mut disputed_tx).unwrap();
        assert!(disputed_tx.in_dispute());

        disputed_tx.dispute();
        let result = user_account.resolve(&resolve_tx, &mut disputed_tx);
        assert!(result.is_ok());
        assert!(user_account.held() == Decimal::ZERO);
        assert!(!disputed_tx.in_dispute());
    }

    #[test]
    fn chargeback_should_lock_account_when_invoked() {
        let mut user_account = ClientState {
            client_id: 123,
            available: Decimal::from_f64(100.).unwrap(),
            held: Decimal::ZERO,
            locked: false,
        };
        let mut disputed_tx = Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 123,
            tx_id: 1,
            amount: Some(Decimal::from_f64(100.).unwrap()),
            in_dispute: false,
        };
        let dispute_tx = Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 123,
            tx_id: 1,
            amount: None,
            in_dispute: false,
        };
        let chargeback_tx = Transaction {
            tx_type: TransactionType::Chargeback,
            client_id: 123,
            tx_id: 1,
            amount: None,
            in_dispute: false,
        };

        let result = user_account.deposit(&disputed_tx);
        assert!(result.is_ok());
        let result = user_account.dispute(&dispute_tx, &mut disputed_tx);
        assert!(result.is_ok());
        assert!(disputed_tx.in_dispute());
        let result = user_account.chargeback(&chargeback_tx, &disputed_tx);
        assert!(result.is_ok());
        assert!(user_account.is_locked());

        let result = user_account.deposit(&disputed_tx);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Account '123' is locked".to_string()
        );
    }
}

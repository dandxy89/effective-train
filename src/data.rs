use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    #[serde(rename = "client")]
    /// Clients are represented by u16 integers
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub tx_id: u32,
    #[serde(rename = "amount")]
    pub amount: Option<Decimal>,
    #[serde(skip_deserializing)]
    pub in_dispute: bool,
}

impl Transaction {
    pub fn tx_id(&self) -> u32 {
        self.tx_id
    }

    pub fn tx_type(&self) -> &TransactionType {
        &self.tx_type
    }

    pub fn client_id(&self) -> u16 {
        self.client_id
    }

    pub fn amount(&self) -> Option<Decimal> {
        self.amount
    }

    pub fn dispute(&mut self) {
        self.in_dispute = true;
    }

    pub fn in_dispute(&self) -> bool {
        self.in_dispute
    }

    pub fn is_disputable(&self) -> bool {
        matches!(
            self.tx_type,
            TransactionType::Deposit | TransactionType::Withdrawal
        )
    }
}

use anyhow::Result;

use crate::data::Transaction;

pub trait Transact {
    fn chargeback(&mut self, tx: &Transaction, chargeback_tx: &Transaction) -> Result<()>;
    fn deposit(&mut self, tx: &Transaction) -> Result<()>;
    fn dispute(&mut self, tx: &Transaction, disputed_tx: &mut Transaction) -> Result<()>;
    fn resolve(&mut self, tx: &Transaction, disputed_tx: &mut Transaction) -> Result<()>;
    fn withdraw(&mut self, tx: &Transaction) -> Result<()>;
}

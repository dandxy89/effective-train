use std::collections::BTreeMap;

use csv_async::{AsyncReader, Trim};
use rust_decimal::{Decimal, RoundingStrategy};
use tokio::fs::File;

use crate::account::ClientState;

/// # Errors
/// If the `file_path` provided does not exist
pub async fn async_read_csv(file_path: &str) -> anyhow::Result<AsyncReader<File>> {
    let file = File::open(file_path).await?;
    Ok(csv_async::AsyncReaderBuilder::new()
        .trim(Trim::All)
        .create_reader(file))
}

fn round_decimal(v: Decimal) -> String {
    v.round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero)
        .to_string()
}

#[allow(clippy::implicit_hasher)]
/// # Errors
/// Can fail to write to `stdout`
pub async fn display_results(results: BTreeMap<u16, ClientState>) -> anyhow::Result<()> {
    let mut writer = csv_async::AsyncWriter::from_writer(tokio::io::stdout());
    writer
        .write_record(&["client", "available", "held", "total", "locked"])
        .await?;

    for (_, client) in results {
        writer
            .write_record(&[
                client.id().to_string(),
                round_decimal(client.available()),
                round_decimal(client.held()),
                round_decimal(client.total()),
                client.is_locked().to_string(),
            ])
            .await?;
    }

    Ok(())
}

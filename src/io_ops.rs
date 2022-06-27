use ahash::AHashMap;
use csv_async::{AsyncReader, Trim};
use futures::stream::StreamExt;
use rust_decimal::{Decimal, RoundingStrategy};
use tokio::{fs::File, sync::mpsc::UnboundedSender};

use crate::{account::ClientState, data::Transaction};

/// # Errors
/// If the `file_path` provided does not exist
pub async fn async_read_csv(file_path: &str) -> anyhow::Result<AsyncReader<File>> {
    let file = File::open(file_path).await?;
    Ok(csv_async::AsyncReaderBuilder::new()
        .trim(Trim::All)
        .create_reader(file))
}

pub async fn partition_csv_events(
    mut reader: AsyncReader<File>,
    event_senders: Vec<UnboundedSender<Transaction>>,
    num: usize,
) -> anyhow::Result<()> {
    let mut records = reader.records();
    while let Some(record) = records.next().await {
        if let core::result::Result::Ok(record) = record {
            let tx = record.deserialize::<Transaction>(None)?;
            event_senders[tx.client_id() as usize % num]
                .send(tx)
                .unwrap();
        }
    }

    Ok(())
}

fn round_decimal(v: Decimal) -> String {
    v.round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero)
        .to_string()
}

#[allow(clippy::implicit_hasher)]
/// # Errors
/// Can fail to write to `stdout`
pub async fn display_results(results: AHashMap<u16, ClientState>) -> anyhow::Result<()> {
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

#![deny(rust_2018_idioms)]
#![deny(clippy::correctness)]
#![deny(clippy::perf)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

use ahash::AHashMap;
use anyhow::Context;
use tokio::sync::mpsc;

use crate::{
    io_ops::{async_read_csv, display_results, partition_csv_events},
    ledger::event_handler,
};

pub(crate) mod account;
pub(crate) mod data;
pub(crate) mod io_ops;
pub(crate) mod ledger;

// https://docs.rs/tokio/latest/tokio/attr.main.html
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // let file_appender = tracing_appender::rolling::never("", "transaction_processor.log");
    // tracing_subscriber::fmt()
    //     .with_ansi(false)
    //     .with_writer(file_appender)
    //     .init();

    // Parse CLI Argument
    let mut args = std::env::args();
    let bin_name = args.next().context("Cannot parse executable name")?;
    let file_path = args
        .next()
        .context(format!("Usage: {bin_name} <transactions.csv>"))?;

    // count logical cores this process could try to use
    let num = num_cpus::get();

    // Instantiate workers and senders
    let (mut event_senders, mut workers) = (Vec::with_capacity(num), Vec::with_capacity(num));
    for _ in 0..num {
        let (client_sender, client_receiver) = mpsc::unbounded_channel();
        event_senders.push(client_sender);
        workers.push(tokio::spawn(event_handler(client_receiver)));
    }

    // Read each line of CSV and push parsed records to Event Router
    let reader = async_read_csv(&file_path).await?;
    partition_csv_events(reader, event_senders, num).await?;

    let mut results = AHashMap::new();
    for event_handler in workers {
        let client_results = event_handler.await?;
        results.extend(client_results);
    }

    display_results(results).await
}

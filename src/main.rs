#![deny(rust_2018_idioms)]
#![deny(clippy::correctness)]
#![deny(clippy::perf)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

use anyhow::Context;

pub(crate) mod account;
pub(crate) mod data;
pub(crate) mod io_ops;
pub(crate) mod ledger;

fn main() -> anyhow::Result<()> {
    let file_appender = tracing_appender::rolling::never("", "transaction_processor.log");
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(file_appender)
        .init();

    // Parse CLI Argument
    let mut args = std::env::args();
    let bin_name = args.next().context("Cannot parse executable name")?;
    let file_path = args
        .next()
        .context(format!("Usage: {bin_name} <transactions.csv>"))?;

    println!("BinName `{bin_name}` and Path `{file_path}`");

    Ok(())
}

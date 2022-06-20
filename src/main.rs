#![deny(rust_2018_idioms)]
#![deny(clippy::correctness)]
#![deny(clippy::perf)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

pub(crate) mod data;
pub(crate) mod ledger;

fn main() {
    println!("Hello, world!");
}

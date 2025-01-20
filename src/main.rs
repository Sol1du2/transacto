use anyhow::Result;
use std::env;

use transacto::accounting::ledger::Ledger;
use transacto::data;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: cargo run -- <input_file>");
        std::process::exit(1);
    }

    let mut ledger = Ledger::new();
    data::process_csv(&args[1], &mut ledger)?;
    data::export_csv(&ledger)?;

    Ok(())
}

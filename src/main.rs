use std::env;

use log::error;

use transacto::accounting::ledger::Ledger;
use transacto::data;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        error!("Usage: cargo run -- <input_file>");
        return;
    }

    let mut ledger = Ledger::new();

    if let Err(err) = data::process_csv(&args[1], &mut ledger) {
        error!("failed to process csv, err={}", err);
        return;
    }

    if let Err(err) = data::export_csv(&ledger) {
        error!("failed to export csv, err={}", err);
    }
}

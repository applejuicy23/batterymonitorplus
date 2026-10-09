//! Print battery levels of connected peripherals.
//!
//!   cargo run -p bmp-battery --bin bmp-probe [-- --json]

use std::time::Instant;

/// Reads all batteries once and prints them as a table, or as JSON with `--json`.
fn main() {
    let json = std::env::args().any(|a| a == "--json");
    let started = Instant::now();
    let readings = bmp_battery::read_all_batteries();

    if json {
        println!("{}", serde_json::to_string_pretty(&readings).unwrap());
        return;
    }
    if readings.is_empty() {
        println!("no device reports a battery level");
    }
    for r in &readings {
        let charging = match r.charging {
            Some(true) => "  charging",
            Some(false) => "  on battery",
            None => "",
        };
        println!("{:>3}%  {:<32} [{}]{charging}", r.percent, r.name, r.id);
    }
    eprintln!("({} ms)", started.elapsed().as_millis());
}

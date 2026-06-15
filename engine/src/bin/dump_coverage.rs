/// Dumps the ability coverage data accumulated during test runs to a JSON file.
/// Run after: cargo test --test run_all
/// Run with:  cargo run --bin dump_coverage
use rabuka_engine::ability::debug;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "coverage_data.json".to_string());
    match debug::write_coverage_json(&path) {
        Ok(()) => log::debug!("Done."),
        Err(e) => {
            log::debug!("Error writing coverage data: {}", e);
            std::process::exit(1);
        }
    }
}

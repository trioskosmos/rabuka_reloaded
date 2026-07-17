// Generates flamegraph.svg from a folded-stacks file produced by the
// `rabuka_engine::timer::print_folded` instrumentation.
//
// Usage: cargo run --example gen_flamegraph -- [input.folded] [output.svg]
// Defaults: folded.txt -> flamegraph.svg

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use inferno::flamegraph::{from_reader, Options};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).cloned().unwrap_or_else(|| "folded.txt".to_string());
    let output = args.get(2).cloned().unwrap_or_else(|| "flamegraph.svg".to_string());

    let infile = File::open(&input)?;
    let reader = BufReader::new(infile);

    let outpath = PathBuf::from(&output);
    let outfile = File::create(&outpath)?;
    let writer = BufWriter::new(outfile);

    let mut opt = Options::default();
    opt.title = "rabuka_engine profile_target (5000 games)".to_string();
    from_reader(&mut opt, reader, writer)?;

    eprintln!("Wrote {}", outpath.display());
    Ok(())
}

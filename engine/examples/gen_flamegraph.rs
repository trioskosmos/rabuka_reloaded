// Generates flamegraph.svg from a folded-stacks file produced by the
// `rabuka_engine::timer::print_folded` instrumentation.
//
// Usage: cargo run --example gen_flamegraph -- [input.folded] [output.svg]
// Defaults: folded.txt -> flamegraph.svg

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use inferno::flamegraph::{from_reader, Options};

type Stacks = BTreeMap<Vec<String>, u64>;

const USAGE: &str = "Usage: gen_flamegraph [input.folded] [output.svg] [--exclusive] [--reverse] [--title TEXT]\nDefaults: folded.txt -> flamegraph.svg; input is inclusive timer nanoseconds.\n--exclusive  Input already contains exclusive nanosecond weights (not sample counts).\n--reverse    Show leaf-first stacks to group costs by callee.\n--title TEXT Override the graph title.\nInput must contain only folded stacks, blank lines, or # comments.";

#[derive(Debug, Default)]
struct Config {
    input: PathBuf,
    output: PathBuf,
    exclusive: bool,
    reverse: bool,
    title: Option<String>,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn parse_args(args: impl IntoIterator<Item = String>) -> io::Result<Option<Config>> {
    let mut config = Config::default();
    let mut paths = Vec::new();
    let mut args = args.into_iter();
    let mut positional = false;
    while let Some(arg) = args.next() {
        if positional {
            paths.push(arg);
            continue;
        }
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--exclusive" => config.exclusive = true,
            "--reverse" => config.reverse = true,
            "--title" => {
                config.title = Some(
                    args.next()
                        .ok_or_else(|| invalid("--title requires text"))?,
                );
            }
            "--" => positional = true,
            _ if arg.starts_with('-') => return Err(invalid(format!("Unknown option: {arg}"))),
            _ => paths.push(arg),
        }
    }
    if paths.len() > 2 {
        return Err(invalid("Expected at most an input and an output path"));
    }
    config.input = paths
        .first()
        .map_or_else(|| "folded.txt".into(), PathBuf::from);
    config.output = paths
        .get(1)
        .map_or_else(|| "flamegraph.svg".into(), PathBuf::from);
    Ok(Some(config))
}

fn parse_stacks(reader: impl BufRead) -> io::Result<Stacks> {
    let mut stacks = Stacks::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let line = if index == 0 {
            line.trim_start_matches('\u{feff}')
        } else {
            &line
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fail = |reason: &str| invalid(format!("Line {}: {reason}", index + 1));
        let split = line
            .rfind(char::is_whitespace)
            .ok_or_else(|| fail("expected stack and nanoseconds"))?;
        let path: Vec<String> = line[..split]
            .trim_end()
            .split(';')
            .map(str::to_owned)
            .collect();
        if path.iter().any(|frame| frame.trim().is_empty()) {
            return Err(fail("empty stack frame"));
        }
        let value = line[split..]
            .trim()
            .parse::<u64>()
            .map_err(|_| fail("expected nonnegative integer nanoseconds fitting u64"))?;
        let entry = stacks.entry(path).or_insert(0);
        *entry = entry
            .checked_add(value)
            .ok_or_else(|| fail("duplicate stack total overflows u64"))?;
    }
    if stacks.is_empty() {
        return Err(invalid("No folded stacks found"));
    }
    Ok(stacks)
}

fn exclusive_stacks(inclusive: &Stacks) -> io::Result<Stacks> {
    let mut exclusive = inclusive.clone();
    for (path, value) in inclusive {
        if path.len() == 1 {
            continue;
        }
        let parent = &path[..path.len() - 1];
        let remaining = exclusive.get_mut(parent).ok_or_else(|| {
            invalid(format!("Missing inclusive parent '{}' for '{}'; use --exclusive only for already-exclusive input", parent.join(";"), path.join(";")))
        })?;
        *remaining = remaining.checked_sub(*value).ok_or_else(|| {
            invalid(format!("Child durations exceed inclusive parent '{}'; capture may be incomplete or have overlapping scopes", parent.join(";")))
        })?;
    }
    Ok(exclusive)
}

fn total_weight(stacks: &Stacks) -> io::Result<u64> {
    let total = stacks
        .values()
        .try_fold(0u64, |total, value| total.checked_add(*value))
        .ok_or_else(|| invalid("Total duration overflows u64"))?;
    if total == 0 {
        return Err(invalid("Capture contains no positive duration"));
    }
    Ok(total)
}

fn inclusive_stacks(exclusive: &Stacks) -> io::Result<Stacks> {
    let mut inclusive = Stacks::new();
    for (path, value) in exclusive {
        for depth in 1..=path.len() {
            let entry = inclusive.entry(path[..depth].to_vec()).or_insert(0);
            *entry = entry
                .checked_add(*value)
                .ok_or_else(|| invalid("Inclusive duration overflows u64"))?;
        }
    }
    Ok(inclusive)
}

fn render(
    config: &Config,
    exclusive: &Stacks,
    total: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut folded = Vec::new();
    for (path, value) in exclusive {
        if *value > 0 {
            writeln!(folded, "{} {}", path.join(";"), value)?;
        }
    }
    let view = if config.reverse {
        "leaf-first"
    } else {
        "caller-first"
    };
    let accounting = if config.exclusive {
        "exclusive input"
    } else {
        "inclusive input corrected"
    };
    let mut opt = Options::default();
    opt.title = config
        .title
        .clone()
        .unwrap_or_else(|| "rabuka_engine instrumented elapsed time".into());
    opt.subtitle = Some(format!("{:.3} ms instrumented coverage | {accounting} | {view} | not CPU samples or whole-run time", total as f64 / 1_000_000.0));
    opt.count_name = "nanoseconds".into();
    opt.hash = true;
    opt.frame_height = 20;
    opt.min_width = 0.0;
    opt.reverse_stack_order = config.reverse;
    let mut svg = Vec::new();
    from_reader(&mut opt, folded.as_slice(), &mut svg)?;
    Ok(svg)
}

fn print_summary(exclusive: &Stacks, total: u64) -> io::Result<()> {
    let inclusive = inclusive_stacks(exclusive)?;
    let mut rows: Vec<_> = exclusive.iter().filter(|(_, value)| **value > 0).collect();
    rows.sort_by(|(path_a, value_a), (path_b, value_b)| {
        value_b.cmp(value_a).then_with(|| path_a.cmp(path_b))
    });
    eprintln!(
        "\nInstrumented coverage: {:.3} ms (not whole-run time)",
        total as f64 / 1_000_000.0
    );
    eprintln!("Top 20 self-time paths; self includes untimed descendants and timer overhead.");
    eprintln!(
        "{:>12} {:>12} {:>8}  Call path",
        "Self ms", "Inclusive ms", "Self %"
    );
    for (path, value) in rows.into_iter().take(20) {
        eprintln!(
            "{:>12.3} {:>12.3} {:>7.2}%  {}",
            *value as f64 / 1_000_000.0,
            inclusive[path] as f64 / 1_000_000.0,
            *value as f64 / total as f64 * 100.0,
            path.join(" -> ")
        );
    }
    eprintln!("Call counts and per-call averages are unavailable in folded timer input.");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(config) = parse_args(std::env::args().skip(1))? else {
        println!("{USAGE}");
        return Ok(());
    };
    let input_path = config.input.canonicalize()?;
    if config.output.exists() && config.output.canonicalize()? == input_path {
        return Err(invalid("Input and output must be different files").into());
    }
    let stacks = parse_stacks(BufReader::new(File::open(&input_path)?))?;
    let exclusive = if config.exclusive {
        stacks
    } else {
        exclusive_stacks(&stacks)?
    };
    let total = total_weight(&exclusive)?;
    let svg = render(&config, &exclusive, total)?;
    let mut writer = BufWriter::new(File::create(&config.output)?);
    writer.write_all(&svg)?;
    writer.flush()?;
    print_summary(&exclusive, total)?;
    eprintln!("Wrote {}", config.output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Stacks {
        parse_stacks(input.as_bytes()).unwrap()
    }

    #[test]
    fn subtracts_immediate_children_only_and_preserves_total() {
        let input =
            parse("root 100\nroot;child 60\nroot;child;leaf 20\nroot;sibling 10\nother 5\n");
        let exclusive = exclusive_stacks(&input).unwrap();
        assert_eq!(
            exclusive,
            parse("root 30\nroot;child 40\nroot;child;leaf 20\nroot;sibling 10\nother 5\n")
        );
        assert_eq!(total_weight(&exclusive).unwrap(), 105);
        assert_eq!(inclusive_stacks(&exclusive).unwrap(), input);
    }

    #[test]
    fn accepts_bom_crlf_comments_spaces_and_duplicate_paths() {
        assert_eq!(
            parse(
                "\u{feff}# capture\r\n\r\nroot frame\t10\r\nroot frame 5\r\nroot frame;child 0\r\n"
            ),
            parse("root frame 15\nroot frame;child 0")
        );
    }

    #[test]
    fn rejects_malformed_input_with_line_numbers() {
        for input in [
            "noise",
            "root -1",
            "root 1.5",
            "root;;child 1",
            "root; 1",
            "root nope",
            "root 18446744073709551616",
        ] {
            assert!(parse_stacks(format!("# header\n{input}").as_bytes())
                .unwrap_err()
                .to_string()
                .contains("Line 2"));
        }
        assert!(parse_stacks("# empty\n".as_bytes()).is_err());
        assert!(parse_stacks("root 18446744073709551615\nroot 1".as_bytes()).is_err());
    }

    #[test]
    fn rejects_incomplete_or_overlapping_inclusive_capture() {
        assert!(exclusive_stacks(&parse("root;child 5"))
            .unwrap_err()
            .to_string()
            .contains("Missing inclusive parent"));
        assert!(exclusive_stacks(&parse("root 10\nroot;a 6\nroot;b 5"))
            .unwrap_err()
            .to_string()
            .contains("exceed"));
    }

    #[test]
    fn handles_recursive_labels_and_fully_nested_time() {
        let input = parse("root 10\nroot;root 10\nroot;root;root 10");
        let exclusive = exclusive_stacks(&input).unwrap();
        assert_eq!(exclusive, parse("root 0\nroot;root 0\nroot;root;root 10"));
        assert_eq!(total_weight(&exclusive).unwrap(), 10);
    }

    #[test]
    fn rejects_zero_and_overflowing_totals() {
        assert!(total_weight(&parse("root 0")).is_err());
        assert!(total_weight(&parse("a 18446744073709551615\nb 1")).is_err());
    }

    #[test]
    fn parses_options_and_preserves_positional_defaults() {
        let default = parse_args(Vec::new()).unwrap().unwrap();
        assert_eq!(default.input, PathBuf::from("folded.txt"));
        assert!(!default.exclusive);
        let args = [
            "input.txt",
            "out.svg",
            "--exclusive",
            "--reverse",
            "--title",
            "Example",
        ];
        let config = parse_args(args.map(str::to_owned)).unwrap().unwrap();
        assert!(config.exclusive && config.reverse);
        assert_eq!(config.title.as_deref(), Some("Example"));
        assert_eq!(config.output, PathBuf::from("out.svg"));
        assert!(parse_args(["--help".into()]).unwrap().is_none());
        for args in [vec!["--bad"], vec!["--title"], vec!["a", "b", "c"]] {
            assert!(parse_args(args.into_iter().map(str::to_owned)).is_err());
        }
    }

    #[test]
    fn renders_corrected_nanoseconds_and_escaped_titles_in_both_views() {
        let exclusive = exclusive_stacks(&parse("root 100\nroot;child 60")).unwrap();
        for reverse in [false, true] {
            let config = Config {
                reverse,
                title: Some("Test <capture> & timing".into()),
                ..Config::default()
            };
            let svg = String::from_utf8(render(&config, &exclusive, 100).unwrap()).unwrap();
            assert!(svg.contains("<svg"));
            assert!(svg.contains("100 nanoseconds"));
            assert!(!svg.contains("160 nanoseconds"));
            assert!(svg.contains("Test &lt;capture&gt; &amp; timing"));
            assert!(svg.contains("not CPU samples or whole-run time"));
        }
    }

    #[test]
    fn exclusive_input_can_omit_parent_records() {
        let exclusive = parse("root;child 60\nroot 40");
        assert_eq!(total_weight(&exclusive).unwrap(), 100);
        assert_eq!(
            inclusive_stacks(&exclusive).unwrap(),
            parse("root 100\nroot;child 60")
        );
    }
}

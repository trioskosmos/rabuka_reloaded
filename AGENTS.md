# AGENTS.md

## Output / logs
- When running tests, do NOT truncate/`Select-Object` the output excessively. Use `$env:RUST_LOG="debug"` in PowerShell to activate the logger and stream all `log::debug!` output during test execution. Capture full output so failures are diagnosable.
- Prefer capturing the full command output to inspect rather than cutting it off after a few lines.
- Do not use `git` to revert or fall back on unless explicitly asked — fix the code properly.

## Tests
- The engine test suite is run from the `engine` directory: `cargo test --test run_all`.
- To run a single module: `cargo test --test run_all <module_name>`.
- To run a single test with debug logs: `$env:RUST_LOG="debug"; cargo test --test run_all <test_name> -- --nocapture`.
- Tests load the real card database from `cards/cards.json` via the baked bytecode; regenerate card abilities with `python ability_extraction/extract_card_abilities.py` from the `cards` directory when the parser changes, then run the engine suite.

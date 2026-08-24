# AGENTS.md

## Output / logs
- **Debug env is for FAILING tests only — it slows runs down.** Full-suite / green-run checks: plain `cargo test --test run_all`. When a failure appears OR you're verifying a fix: re-run ONLY the failing tests with `$env:RUST_LOG="debug"; cargo test --test run_all <failing_substring> -- --nocapture --test-threads=1`. Note cargo accepts ONE positional filter — pick a substring matching the tests you need.
- Redirect output to a file (`| Out-File -Encoding utf8 <file>`) and inspect it with the Read tool. No Select-String surgery on live console output; don't truncate.
- When ANY assertion fails, read the condition-verdict/trace lines from that debug output BEFORE changing code or tests. Never guess at engine internals.
- Do not use `git` to revert or fall back on unless explicitly asked — fix the code properly.

## Tests
- The engine test suite is run from the `engine` directory: `cargo test --test run_all`.
- To run a single module: `cargo test --test run_all <module_name>` (still with `$env:RUST_LOG="debug";`).
- Tests load the real card database from `cards/cards.json` via the baked bytecode; regenerate card abilities with `python ability_extraction/extract_card_abilities.py` from the `cards` directory when the parser changes, then run the engine suite. Note: `condition_decoder_gen.rs` / `effect_decoder_gen.rs` are AUTO-GENERATED (see their headers) — edit `cards/generate_condition_decoder.py` / the generator, never the generated file.
- Coverage inventory is automated: `python cards/test_inventory.py` regenerates `engine/tests/TEST_COVERAGE.md`, `docs/ABILITY_MATRIX.md`, `engine/tests/TEST_INVENTORY.{json,md}` from `cards/abilities.json` + `engine/tests`. CI checks it via `python cards/test_inventory.py --check`. Do not hand-edit the generated markdown.

## Work habits
- Before writing a new gameplay test, search existing tests for similar ability text (same clause shapes) and copy the setup/drain/assert idiom.
- When a test fails, classify: test bug (wrong expectation/setup), engine bug (fix engine + keep test), or parser gap (regenerate + golden-diff). State which in the commit message.

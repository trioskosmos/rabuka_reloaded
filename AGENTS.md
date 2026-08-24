# AGENTS.md

## Output / logs
- **⛔ NEVER TRUNCATE OUTPUT — THIS MEANS YOU.** Do NOT filter live console output with Select-String / Select-Object -First/-Last chains instead of reading it. That hides exactly the lines needed and wastes turns guessing. THE ONLY ACCEPTABLE FLOW: run command → full output to file (`| Out-File -Encoding utf8 <file>`) → **Read tool** on that file (offset/limit for big files). Post-read grep on the FILE is fine; console-filtering INSTEAD of reading is a mistake. If you catch yourself writing `Select-String` on console output of a failing run, stop and redo it via file+Read.
- **⛔ NEVER write throwaway Python/PowerShell scripts to patch files.** Use the Edit tool with exact strings read from the file. Script-generated edits are opaque, fail silently on mismatch, and leave litter behind. Read the region, then Edit it directly.
- **Debug env (`$env:RUST_LOG="debug"`) is for FAILING tests only — it slows full runs down.** Green full-suite checks: plain `cargo test --test run_all`. When diagnosing/verifying: `$env:RUST_LOG="debug"; cargo test --test run_all <failing_substring> -- --nocapture --test-threads=1`. Cargo takes ONE positional filter — pick a substring covering the tests you need.
- When ANY assertion fails, read the condition-verdict/trace lines from the debug file BEFORE changing code or tests. Never guess at engine internals.
- Do not use `git` to revert or fall back on unless explicitly asked — fix the code properly.

## Tests
- The engine test suite is run from the `engine` directory: `cargo test --test run_all`.
- To run a single module: `cargo test --test run_all <module_name>` (debug env only when diagnosing).
- Tests load the real card database from `cards/cards.json` via the baked bytecode; regenerate card abilities with `python ability_extraction/extract_card_abilities.py` from the `cards` directory when the parser changes, then run the engine suite. Note: `condition_decoder_gen.rs` / `effect_decoder_gen.rs` are AUTO-GENERATED (see their headers) — edit `cards/generate_condition_decoder.py` / the generator, never the generated file.
- Coverage inventory is automated: `python cards/test_inventory.py` regenerates `engine/tests/TEST_COVERAGE.md`, `docs/ABILITY_MATRIX.md`, `engine/tests/TEST_INVENTORY.{json,md}` from `cards/abilities.json` + `engine/tests`. CI checks it via `python cards/test_inventory.py --check`. Do not hand-edit the generated markdown.

## Work habits
- Before writing a new gameplay test, search existing tests for similar ability text (same clause shapes) and copy the setup/drain/assert idiom.
- When a test fails, classify: test bug (wrong expectation/setup), engine bug (fix engine + keep test), or parser gap (regenerate + golden-diff). State which in the commit message.

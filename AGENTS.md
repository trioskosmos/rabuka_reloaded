# AGENTS.md

## Output / logs
- **⛔ NEVER TRUNCATE OUTPUT — THIS MEANS YOU.** Do NOT filter live console output with Select-String / Select-Object -First/-Last chains instead of reading it. That hides exactly the lines needed and wastes turns guessing. THE ONLY ACCEPTABLE FLOW: run command → full output to file (`| Out-File -Encoding utf8 <file>`) → **Read tool** on that file (offset/limit for big files). Post-read grep on the FILE is fine; console-filtering INSTEAD of reading is a mistake. If you catch yourself writing `Select-String` on console output of a failing run, stop and redo it via file+Read.
- **⛔ NEVER write throwaway Python/PowerShell scripts to patch files.** Use the Edit tool with exact strings read from the file. Script-generated edits are opaque, fail silently on mismatch, and leave litter behind. Read the region, then Edit it directly.
- **Debug env (`$env:RUST_LOG="debug"`) is for FAILING tests only — it slows full runs down.** Green full-suite checks: plain `cargo test --test run_all`. When diagnosing/verifying: `$env:RUST_LOG="debug"; cargo test --test run_all <failing_substring> -- --nocapture --test-threads=1`. Cargo takes ONE positional filter — pick a substring covering the tests you need.
- When ANY assertion fails, read the condition-verdict/trace lines from the debug file BEFORE changing code or tests. Never guess at engine internals.
- **READ WHOLE FLOWS.** When debugging, Read the ENTIRE debug file and the ENTIRE relevant source functions (not 10-line snippets) before forming a hypothesis. Trace the full path: trigger → condition eval → every sub-action → placement. One-line-at-a-time grepping that skips intermediate frames leads to wrong root causes — if two consecutive probes disagree, you skipped something.
- **ADD PERMANENT `log::debug!` LINES for anything you have even slight doubts about** (condition verdicts, branch taken, computed values, decoded fields). These stay in the code as first-class diagnostics — they cost nothing when RUST_LOG is off and turn the next debugging session into a file read instead of an archaeology dig. Do not remove them when the bug is fixed; they are part of the fix.
- Do not use `git` to revert or fall back on unless explicitly asked — fix the code properly.

## Tests
- The engine test suite is run from the `engine` directory: `cargo test --test run_all`.
- To run a single module: `cargo test --test run_all <module_name>` (debug env only when diagnosing).
- Tests load the real card database from `cards/cards.json` via the baked bytecode; regenerate card abilities with `python ability_extraction/extract_card_abilities.py` from the `cards` directory when the parser changes, then run the engine suite. Note: `condition_decoder_gen.rs` / `effect_decoder_gen.rs` are AUTO-GENERATED (see their headers) — edit `cards/generate_condition_decoder.py` / the generator, never the generated file.
- Coverage inventory is automated: `python cards/test_inventory.py` regenerates `engine/tests/TEST_COVERAGE.md`, `docs/ABILITY_MATRIX.md`, `engine/tests/TEST_INVENTORY.{json,md}` from `cards/abilities.json` + `engine/tests`. CI checks it via `python cards/test_inventory.py --check`. Do not hand-edit the generated markdown.

## Work habits
- Before writing a new gameplay test, search existing tests for similar ability text (same clause shapes) and copy the setup/drain/assert idiom.
- When a test fails, classify: test bug (wrong expectation/setup), engine bug (fix engine + keep test), or parser gap (regenerate + golden-diff). State which in the commit message.
- **Gaps are FIXED end-to-end, never documented around.** A parser gap means: add the parser pattern, regenerate `abilities.json` + bytecode, add the engine evaluation branch if missing, and make the test assert the card's full printed behavior. NEVER convert a failing test to `#[ignore]`, delete it, or weaken its assertions to pin broken/partial behavior. Documenting a gap in the audit doc is a complement to fixing it, not a substitute.

## Debugging patterns learned (2026-09-10)
- **Picker routing bug**: `execute_reveal_effect` calls `execute_reveal` but didn't set `current_effect` first. The `picker` field from the parsed effect was never accessible when building the Choice. Fix: set `current_effect = Some(effect.clone())` in the caller before delegating.
- **Choice data flow**: `current_effect` → `ChoiceBuilder.picker()` → `Choice::SelectCard.picker` → PCA_G1 routing. If any link is missing, routing falls back to legacy `target_player_id` logic.
- **Preserve executor intent**: `execute_choice` sets `choice_player_id` via `choice_maker`. PCA_G1 must NOT overwrite if already set.
- **Debug trace reading**: Look for `picker=None` at PCA_G1 — means data lost between choice creation and routing. Trace backward: choice creation → current_effect → executor.
- **Fix at source, not sink**: Don't add complex routing logic; ensure the data is present where the choice is built.

## Failure points learned (2026-09-17, test re-sort + Toubatsu incident)
- **⛔ VERIFY CARD IDENTITY against `cards/cards.json` before concluding anything.** `bp2` vs `pb2` (etc.) are DIFFERENT CARDS that share character + number — e.g. `PL!SP-bp2-011-R` (debut-only) vs `PL!SP-pb2-011-R` (auto + live-start). A test staging the wrong print tests nothing it claims. When a test's behavior contradicts its card's text, read the card's ACTUAL entry (name, cost, abilities) first — do not theorize about engine internals until identity is confirmed.
- **⛔ The `similar_cards` quality prompt is a wrong-card detector — heed it.** Confusable numbers (bp2/pb2) staged in one file mean the test may drive a different card than its header claims. When auditing such files, assert the staged card's identity (`card_no` of the played id) or fix the number.
- **⛔ Check `git log`/status for concurrent commits when results stop making sense.** The tree can move under you mid-session (a commit landing between your green run and your red run invalidates every assumption). `git log --oneline -5` + `git status --short` BEFORE bisecting.
- **⛔ Establish the failing baseline BEFORE editing.** When a test fails after your change, first prove it passed before it: `git stash` the file (or checkout HEAD copy) and run. If it failed before too, your change is innocent — do not bisect your own diff.
- **⛔ "Identical setup" claims must be mechanically diffed, never eyeballed.** A stale copy-paste (wrong card number, leftover line) reads as identical until diffed. Extract both bodies and compare before theorizing about nondeterminism or globals.
- **⛔ No pool/global theories without a direct probe.** If you suspect shared/nondeterministic state (`id()` pools, `PRELOADED`, `RandomState`), write the minimal probe (same lookups, print resolved `card_no`s) and run it 3x before building any theory on top.
- **Environment facts (Windows PowerShell 5.1):** no heredocs (`<<`); quotes are stripped when passing args to NATIVE exes (`python -c "..."` arrives unquoted — pipe via stdin or use files); the Write tool saves scripts without BOM so non-ASCII literals mangle — keep PS scripts ASCII-only (code points via `[char]`) and route output through `Out-File -Encoding utf8` + Read tool.

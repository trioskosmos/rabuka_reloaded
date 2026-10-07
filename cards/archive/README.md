# Archive

One-off investigation scripts and historical artifacts, kept for provenance.
They are NOT part of any pipeline — see `cards/` for the live tooling.

- The `.py` scripts were each written for a single audit. Most hardcode the
  original checkout's absolute path (`C:\Users\trios\...`), so they run
  as-is only on that machine; `describe_fidelity_report.py` uses
  CWD-relative paths instead.
- `cards.json.bak-*` are pre/post-PR backups of `cards.json`.
- `*.log` are run logs.

Documented tools that still live in `cards/` (do not archive — docs and CI
reference them by path): `test_inventory.py` (CI coverage gate),
`gen_group_tree.py`, `jidou_test_audit.py`, `pipeline_report.py`,
`normalise_names.py`, `deck_compression.py`, `audit_field_coverage.py`,
`audit_fixture_annotations.py`, `audit_id_in_assert.py`,
`audit_vs_official.py`, `gen_pbl02_card_table.py`, `_resolve.py`,
and the build/data pipeline (`compile_abilities.py`, `compile_cards.py`,
`generate_effect_decoder.py`, `generate_condition_decoder.py`,
`build_lib.py`, `generator_types.py`, `scrape_*.py`).

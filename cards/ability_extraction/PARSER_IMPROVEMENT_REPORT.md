# Parser Improvement Report

This note captures the current state of the ability parser and where it still feels weakest.

Important: the older markdown docs in `cards/ABILITY_SPEC.md` and `cards/ABILITY_FIELD_MAPPING.md` are fairly old now. They are useful as historical references, but they should not be treated as the current source of truth without cross-checking against the parser, the QA scripts, and the actual `abilities.json` output.

## What the parser is trying to do

The parser is not just extracting text. It is trying to convert Japanese ability text into a structured effect tree that the engine can execute.

That means it needs to recover:

- trigger and use-limit information
- cost vs effect boundaries
- source, destination, and target roles
- action type
- conditions and temporal clauses
- per-unit scaling
- sequential and conditional sub-effects
- parenthetical clarifications

That is a hard problem, and the current implementation already does a lot of useful work. The main issue is that some of the remaining logic still behaves like a keyword matcher when it should behave more like a clause interpreter.

## What is working reasonably well

- The parser already recognizes many common effect families.
- It has structural handling for sequential, conditional, and optional patterns.
- It supports per-unit logic and duration handling.
- It has normalization and post-processing rather than relying on a single giant regex pass.
- There are separate validation and QA scripts, which is a good sign that the team is already trying to measure parser quality instead of guessing.

## Main gaps and weaknesses

### 1. Source and destination inference is still too shallow

This is the biggest weakness.

The parser still leans on broad substring checks for things like `手札`, `デッキ`, `控え室`, and `ステージ`. That works for obvious cases, but it is fragile when the same noun appears in both source and destination contexts.

Examples of where this breaks down:

- `デッキから手札に加える` should be read as source = deck, destination = hand.
- `手札を1枚控え室に置く` should be read as source = hand, destination = discard.
- `手札に加える` should not automatically imply hand as the source just because `手札` appears in the phrase.

This is a structural problem, not just a missing phrase.

### 2. The parser still falls back too often to generic or repaired output

There are many places where the parser gets close, then leaves the result too generic or relies on later cleanup scripts to patch it.

That makes the system harder to trust because:

- the parser output is not always the final semantic interpretation
- the fixer scripts become a second parser in practice
- QA can report issues, but the root cause is still buried in the extraction logic

If the parser is supposed to be the source of truth, it needs to own more of the semantics directly.

### 3. There is overlap between parser, fixer, and validator responsibilities

The current workflow spreads similar logic across several files:

- `cards/ability_extraction/parser.py`
- `cards/ability_extraction/fix_abilities.py`
- `cards/validate_abilities.py`
- `cards/find_dropped_details.py`
- `cards/ability_extraction/deep_qa.py`

That is useful for analysis, but it also means the same inference can be implemented in more than one place.

The risk is:

- inconsistent behavior between scripts
- silent drift over time
- difficulty understanding which layer is authoritative

### 4. The parser is not fully grammar-first yet

The code has a lot of structure, but the remaining weak spots show that it still depends on phrase spotting more than clause interpretation.

Japanese ability text often encodes meaning through:

- `から` for source
- `に` / `へ` / `で` for destination or location
- `場合`, `とき`, `なら` for conditions
- `につき`, `ごとに` for per-unit scaling
- `その後`, `代わりに` for chaining or substitution
- quoted names and quoted ability text

The parser understands many of these already, but not consistently enough to make the structure feel stable.

### 5. Many special cases look like patches for observed examples

Some branches are clearly there because a specific text shape showed up in the data.

That is normal early on, but the parser is now at the point where many of those special cases should probably be promoted into reusable pattern families.

Examples:

- reveal / select / discard-rest chains
- optional-cost-plus-effect patterns
- parenthetical activation restrictions
- timing prefixes like end-of-turn or end-of-live

These are recurring concepts, not one-off exceptions.

### 6. The docs are behind the implementation

The older markdown docs are useful, but they are not current enough to rely on blindly.

In particular:

- `cards/ABILITY_SPEC.md` looks like a generated report snapshot
- `cards/ABILITY_FIELD_MAPPING.md` reads like a mid-stream analysis document
- both are older than the parser behavior now present in `cards/ability_extraction/parser.py`

So if someone uses those docs alone, they may underestimate what the parser already supports or misunderstand what is still missing.

## Specific improvement opportunities

### A. Make source/destination extraction clause-aware

This should be the first priority.

Instead of treating any mention of `手札` or `デッキ` as a source hint, the parser should:

- prefer explicit `から` clauses for source
- prefer `に置く`, `に加える`, `に送る`, `に登場させる` for destination
- treat broad nouns only as fallback evidence

### B. Add a stronger intermediate representation

The parser would benefit from a more explicit internal layer where it can say:

- this is a source phrase
- this is a destination phrase
- this is a condition phrase
- this is a temporal phrase
- this is a list / choice / alternative

That would reduce the amount of text guessing that happens later.

### C. Consolidate repeated vocabulary tables

There are many hardcoded phrase lists for:

- sources
- destinations
- card types
- temporal markers
- operators
- duration prefixes

These should probably be centralized so they do not drift apart.

### D. Treat fixer scripts as migration tools, not permanent parser logic

The fix scripts are useful, but they should not become a hidden second parser.

The parser should own the semantics.
The fixer should only repair older data or run one-off migrations.

### E. Add a small regression set of Japanese examples

The parser needs a compact set of examples that are checked every time:

- deck to hand
- hand to discard
- reveal then select
- per-unit score or cost modifications
- parenthetical activation restrictions
- conditional alternatives
- result-based conditions

That would catch regressions much earlier than manual QA.

### F. Track inference provenance

This would make debugging much easier.

For example, a field could internally know whether it came from:

- explicit source text
- inferred clause role
- parenthetical clarification
- fallback keyword match

That would make it much clearer which parts of the parse are reliable and which parts are best-effort.

## Recommended priority order

1. Tighten source/destination extraction.
2. Reduce overlap between parser, fixer, and validator behavior.
3. Add regression tests for the most common Japanese clause shapes.
4. Promote recurring special cases into reusable structural patterns.
5. Add provenance or confidence tracking for inferred fields.
6. Refresh the older markdown docs or mark them clearly as historical snapshots.

## Bottom line

The parser is already doing real work, and it is more advanced than a plain regex script.

What is still missing is not raw coverage so much as consistency:

- more clause-aware interpretation
- fewer heuristic overlaps
- fewer post-hoc repairs
- more trustworthy semantics for source, destination, and nested conditions

If the goal is to make Japanese ability text behave like a first-class AST, the next step is to make the parser more grammar-driven and less dependent on broad substring inference.

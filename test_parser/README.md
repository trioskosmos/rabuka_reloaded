# Rabuka Reloaded - Ability Parser v2

This directory contains the modernized, production-ready extraction system for parsing Japanese TCG ability text into structured, engine-readable JSON.

## Core Files

The parser is contained entirely within three tightly coupled files:

1. **`grammar.py`**: A minimal Parsing Expression Grammar (PEG) combinator library. It provides the foundational logic (`Seq`, `OneOf`, `Opt`, `Many`, `Map`) for defining complex syntax rules without relying on heavily nested regex.
2. **`models.py`**: Pydantic schemas that strictly define the valid shapes of `Action` and `Cost` objects. This ensures 100% type safety and structure consistency.
3. **`parser_v2.py`**: The main orchestration file. It contains the actual Japanese grammar rules specific to the game (e.g., `gain_resource_effect`, `wait_cost`) and maps them into the Pydantic models.

## Architectural Improvements over `parser.py`

The original `parser.py` (and the `annotator.py`/`mapper.py` experiments) utilized a heuristic, order-dependent regex cascade. This approach had fatal flaws:
- **Brittle Replacements**: `parser.py` relied on `string.replace()` and hardcoded `.split('。')` logic. If a card introduced a period inside a parenthetical `（ウェイト状態...）`, the parser would shatter the sentence improperly.
- **Dynamic JSON Structures**: The old parser dynamically built dictionaries `p['count'] = 1`. If a regex failed to capture a field, it produced malformed JSON that crashed the Rust engine during deserialization.
- **Infinite Whack-a-Mole**: Ordering rules was impossible (e.g., ensuring "デッキの下から" didn't accidentally trigger the simpler "デッキから" rule).

### The v2 Paradigm
The v2 parser solves this by combining **PEG Combinators** with **Strict Schema Validation**:
1. **Context-Free Parsing**: By using `Seq(cost, condition, effects)`, the parser natively understands the *structure* of an ability without needing to split strings manually.
2. **Deterministic Fallbacks**: If the parser encounters a completely unknown phrasing, it doesn't crash or output garbage text. It safely wraps the text in an `UnknownAction` block. This guarantees 100% successful JSON output.
3. **Type Safety**: Everything is validated through Pydantic. The output is guaranteed to align with the Rust engine's `serde` structures.

## A Note on Japanese NLP

It is generally **unnecessary** (and often counterproductive) to use dedicated Japanese NLP tools (like MeCab, Sudachi, or Kuromoji) for parsing TCG ability texts. 

TCG text is a heavily formalized sub-language (Templated Natural Language). NLP tokenizers are trained on general colloquial Japanese and often aggressively split or misinterpret TCG-specific keywords. For example, a tokenizer might split "バトンタッチ" or "ポジションチェンジ" incorrectly. 

Because TCG phrasing is extremely rigid ("XをY枚Zする"), a PEG grammar using exact literal strings (`Str()`) and basic regex (`Re()`) is infinitely more accurate, deterministic, and faster than relying on semantic tokenization.

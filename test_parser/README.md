# Japanese Ability Parser (Hardened PEG Version)

This directory contains a production-ready, deterministic Japanese card ability parser. It replaces the previous heuristic-based extraction with a robust **Bag of Sentences** PEG (Parsing Expression Grammar) architecture.

## Features

- **Deterministic PEG Engine**: Uses a formal grammar to decompose Japanese text into structured components.
- **Bag of Sentences Architecture**: Flexible enough to handle Japanese word order while maintaining strict JSON output.
- **Japanese Text Capture**: Every parsed component (Cost, Condition, Action) includes the **actual raw Japanese text** from the card ability in its `text` field.
- **Pydantic Validation**: All output is strictly validated against the game engine's schema.
- **Official Benchmarking**: Tested against the official **645 unique abilities** from the game database.
- **Noise Resilience**: Gracefully skips unparseable segments within a sentence while capturing valid structural data around them.

## Data Structure

The parser outputs an `Ability` model containing:

- `cost`: Energy costs or action-based costs (e.g., "Discard 1 card:").
- `condition`: Logical checks (e.g., "If you have 2 or more cards in X").
- `effects`: A list of structured `Action` objects.
- `raw_text`: The full original ability text.

### Example Output

```json
{
  "full_text": "{{toujyou.png|登場}}自分のステージにメンバーが2人以上いる場合、カードを1枚引く。",
  "parsed": {
    "cost": null,
    "condition": {
      "text": "自分のステージにメンバーが2人以上いる場合",
      "type": "comparison",
      "target": "自分",
      "location": "ステージ",
      "count": 2,
      "unit": "人"
    },
    "effects": [
      {
        "text": "カードを1枚引く",
        "type": "draw_card",
        "count": 1
      }
    ]
  }
}
```

## Core Components

1.  **`grammar.py`**: A lightweight PEG parsing library with support for `Seq`, `OneOf`, `Many`, `Opt`, and `Capture`.
2.  **`patterns.py`**: Centralized lexicon containing TCG-specific regexes for units, zones, and complex Japanese verb stems.
3.  **`parser_v2.py`**: The main parsing logic implementing the handler cascades and field enrichment.
4.  **`models.py`**: Pydantic schemas for strict type safety.

## Accuracy Metrics

Currently achieving **73.6% full parsing coverage** for the 645 unique ability strings in `abilities.json`. Remaining unknown segments are safely encapsulated in `UnknownAction` containers for engine fallback.

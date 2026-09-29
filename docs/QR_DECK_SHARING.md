# QR Code Deck Sharing

> **Corrected 2026-09-29.** The "How it works" section below previously claimed
> decks are exported as plain `card_no x QTY` text. That is no longer true and
> has not been for some time: the web UI now exports a **compact binary
> encoding, base64-wrapped** for the QR payload. The plain-text format is still
> accepted on *import* only. The QR-version arithmetic lower down was also wrong
> (see "Compression analysis"); the 3DS camera/quirc half of this document is
> accurate and unchanged.

## How it works

A deck of 72 cards (48 members + 12 live + 12 energy) is encoded as a compact
**binary** blob — one byte per card, indexed into the card registry, with a small
header — and then base64-encoded for QR safety. A 30-line deck goes from ~580
bytes of plain text to ~91 bytes of raw binary. See `card_browser.html` around
the `Show QR code for current deck (binary index format)` handler, and
`cards/deck_compression.py` for the capacity model.

The payload is packed into a QR code, displayed on screen (web UI) or scanned by
the 3DS rear camera. The legacy plain-text `card_no x QTY` format remains valid
as an *import* format:

```
Pl!-sd1-001-SD x 4
Pl!-sd1-002-SD x 3
...
```

## Web UI — Generate QR from a deck

**File:** `web_ui/card_browser.html` → "Show QR" button

The deck is exported in the binary format above, then passed to `qrcode.min.js` (local JS library) which renders it as a QR code in a modal window. The QR is 200×200px, error correction level M. The version is now a function of the binary payload size, not of a 500-byte text estimate — see the corrected table under "Compression analysis".

### Import methods (all supported)

The card browser and deck converter accept these formats:

| Method | Example | How |
|--------|---------|-----|
| **Deck Log HTML** | `<span title="PL!-bp3-012-RM : 南 ことり"><span class="num">3</span>` | Paste HTML source (Ctrl+U, Ctrl+A, Ctrl+C) from decklog.bushiroad.com |
| **Official site recipe** | `<a href="...cardno=PL!N-bp3-030-L">...<span>×</span><span>2</span>` | Paste HTML from the official Bushiroad Love Live site |
| **`ID x QTY` text** | `Pl!-sd1-001-SD x 4` | One per line |
| **`QTY x ID` text** | `4 x Pl!-sd1-001-SD` | One per line |
| **Bare IDs** | `Pl!-sd1-001-SD` (repeated for quantity) | One card ID per line |
| **QR code** | Scanned by 3DS camera | Decodes to the plain text above |

## 3DS — Scan QR code

**Menu:** "QR (3DS etc.)" from the main menu

The 3DS uses:
- **Camera:** `cam:u` service via libctru (same API used by FBI)
- **Decoder:** `quirc` QR recognition library (same library used by FBI)
- **Flow:** Init camera → capture 400×240 frame → convert RGB565 to grayscale → run quirc → extract decoded text → validate as deck cards

Source: `platforms/3ds/src/ctru_shim.c` (`_3ds_qr_start`, `_3ds_qr_stop`, `_3ds_qr_poll`)
QR library: `platforms/3ds/src/quirc.c`, `identify.c`, `decode.c`, `version_db.c`

FBI, the most popular 3DS homebrew title manager, uses the identical quirc library for its QR-based CIA installation. The FBI source is at:
[https://github.com/Steveice10/FBI/tree/master/source/libs/quirc](https://github.com/Steveice10/FBI/tree/master/source/libs/quirc)

FBI's camera capture code (`capturecam.c`) was used as a reference for improving the QR scanner's stability. Key improvements over the original implementation:
- Buffer error event handling (prevents camera hang on buffer overflow)
- `GSPGPU_FlushDataCache` after frame copy (ensures GPU sees updated data)
- Mutex timeout instead of indefinite wait (prevents deadlock if camera thread crashes)
- Frame-skipping for QR decoding (reduces main thread load)
- Corrected texture pixel mapping (fixes out-of-bounds GPU memory writes that caused crashes)

## GitHub Pages static site

The web UI in `web_ui/` can be hosted as a GitHub Pages static site **from this same repo**:

1. Go to Settings → Pages
2. Source: "Deploy from a branch"
3. Branch: `master`, folder: `/web_ui`
4. The site will be at `https://<user>.github.io/rabuka_reloaded/`

All deck import/export and QR generation works client-side with no server needed. The only server-dependant feature is the game API (`/api/`) which is the Rust web server — the card browser doesn't use it.

### Files needed for the static site

```
web_ui/
├── index.html          # Main game UI (not needed for card browser alone)
├── card_browser.html   # Deck builder with QR export + all import methods
├── deck_converter.html # Standalone deck format converter
├── tutorial.html       # Guide for importing from Deck Log
├── card_browser.html   # Card browser + deck creator with QR
├── js/
│   ├── qrcode.min.js   # QR code generator (MIT, bundled locally)
│   ├── i18n/           # Localization files
│   └── ...             # Other game JS files
├── css/
├── img/
└── ...
```

## Binary data format

> The prose below previously claimed the text format was "~500 bytes, fitting QR
> version 7 (45×45)" and that a 144-byte payload is "version 4 (33×33)". Both
> were wrong. At error-correction level M, v7 capacity is **196 bytes**, not 500,
> and a 33×33 module grid is **version 3**, not 4. The auto-generated table
> immediately below was always right; the prose contradicted it.

A deck is 72 cards × 2 bytes (u16 database index) = 144 bytes, which needs
roughly QR version 5–6 at level M. The shipped web UI actually uses the tighter
binary-index format (~91–93 bytes for a typical deck), landing at version 4.
The legacy text format (`card_no x QTY`, ~500 bytes) needs version **13** at
level M, per the generated table. All of these are scannable by the 3DS camera.

### Compression analysis (auto-generated)

Run `python cards/deck_compression.py` to regenerate these numbers from `cards/cards.json`.

| Encoding | Size | QR Version | Modules |
|----------|------|------------|---------|
| Text (`card_no x QTY`) | ~500 bytes | 13 | 73×73 |
| Fixed-width binary (u16 index per card) | 93 bytes | 4 | 37×37 |
| Information-theoretic minimum | 57 bytes | 3 | 33×33 |
| Rarity-stripped (base card_no only, energy forced default) | 72 bytes | 4 | 37×37 |

- **2280 unique cards** in database (1356 member, 254 live, 670 energy)
- **816 unique base member card_nos**, **202 unique base live card_nos**, **534 unique base energy card_nos** (after stripping rarity suffix)
- **Max 4 copies** per card number (across rarities)
- **Rarity-stripped** uses ceil(log2(unique_base_count)) bits per card slot; energy cards cost 0 bits (forced to default)
- **8-letter passwords cannot encode decks** — 26⁸ = 208,827,064,576 combinations (37.6 bits) vs 449 bits needed. Server-side lookup key required.

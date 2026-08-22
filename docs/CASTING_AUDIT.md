# Casting Audit: why the engine is full of `as u8` / `as usize`, and how to remove it

*Generated 2026-08-23. Counts from ripgrep over `engine/`.*

## TL;DR

The engine has **~1,034 unchecked integer cast sites**, dominated by `as u8` (333) and
`as usize` (267), with only **5 defensive `try_from` calls** in the whole crate.
There are no lints configured against casting (`cast_possible_truncation`,
`cast_sign_loss`, etc.) — nothing stops a silent wraparound today.

The casts are not random noise; they cluster into a handful of structural causes:

1. **There is no `CardId` type.** Card identity is a bare `i16` everywhere, so it gets
   bounced to `usize` for indexing and `u8` for counters at every zone boundary.
2. **Binary blob / bytecode decoding** in `card_binary.rs` / `vm.rs` reads raw bytes.
3. **A clamp idiom** (`expr.max(0) as u8`) — compute in `i32`, then narrow.
4. **Modifier state keyed by raw `i16`** stored as `i16`, forcing sign conversions.
5. Legit float math in the bot, RNG, and profiling code.

## Where the casts live

| Target type | Sites | Main culprits |
|---|---|---|
| `as u8` | 333 | zone counts, bytecode decode, condition checks |
| `as usize` | 267 | indexing arrays/maps with card IDs |
| `as i16` | ~150 | `GameModifiers` storage |
| `as i32` | ~100 | overflow-safe arithmetic then narrowing |
| `as u32` | ~60 | RNG, byte packing, timers |
| `as f64`/`f32` | ~40 | bot normalization, win-rate/UCT math, profiling |

## The root causes, with receipts

### 1. No ID newtype — the biggest one

Card identity is a raw `i16` throughout (`engine/src/core/card.rs:349-353`):

```rust
pub struct CardDatabase {
    pub cards: HashMap<i16, Card>,
    pub card_no_to_id: HashMap<String, i16>,
    pub next_id: i16,
}
```

Every time an ID is used as an index or stored as a count, someone pays an `as usize`
or `as u8` toll. The only numeric newtype in the crate is `AbilityRef(pub u16)`
(`ability/ability_store.rs:18-19`) — proof the pattern works here already.

**Fix:** introduce `#[derive(...)] pub struct CardId(i16)` with explicit accessors,
plus `From<CardId> for usize`. The compiler then forces every conversion site into
one auditable place, and IDs can never be accidentally used as arithmetic values.

### 2. Binary decoding (`card_binary.rs`, `vm.rs`)

Reading baked bytecode means `bytes[i] as u16`, LEB-style varint assembly, etc.

**Fix:** use `u8::from_le_bytes` / `[u8]::try_into()` at the read sites, and wrap the
decoder so the *only* unchecked casts in the crate live behind one `decode_*` module
with round-trip tests. These casts are actually fine — they just shouldn't be smeared
across the codebase.

### 3. The clamp idiom — `x.max(0) as u8`

Recurring pattern in `condition/card.rs`, `turn/live.rs`, `move_cards.rs`: compute a
count in `i32` (because subtraction can go negative), clamp, then narrow to `u8`.

```rust
let n = (something_i32).max(0) as u8;
```

**Fix:** a tiny helper makes intent explicit and kills the cast noise:

```rust
fn saturate_u8(v: i32) -> u8 { v.clamp(0, u8::MAX as i32) as u8 }
```

One cast inside a tested helper beats three hundred scattered ones. Better still:
stop storing counts as `u8` at all (see #4).

### 4. Modifier values stored as `i16`

`GameModifiers` uses `HashMap<i16, i16>` for bonuses, forcing `as i16` on every write
(`core/game_state/modifiers.rs:144,147,156,159`; `core/game_modifiers.rs:663-699`;
`ability/effects/ability_effects.rs:381,392`). Some paths even do
`i32 → i16 → i32` double conversion.

**Fix:** store modifiers as `i32` (or make keys `CardId` and values `i32`). Memory
cost is negligible outside embedded targets; check whether the GBA/3DS platforms
actually need the narrowing — if so, confine it to the platform serialization layer.

### 5. Float casts in bot/stats — mostly legitimate

`bot/neural.rs` (17×), strategy/ISMCTS win-rate math, `timer.rs` profiling:
`as f64` from integers is normal and lossless up to 2^53. **Leave these alone**, or
at most adopt `f64::from(x)` where the source type is unambiguous.

### 6. Enum discriminant casts

Almost everything matches explicitly (good). One outlier: `HeartColor::index()`
(`core/card.rs:3878`) feeding `required_arr[color.index()] = me.set as u8;`
in `turn/live.rs:315-405`.

**Fix:** give `HeartColor` an explicit discriminant + `from_index` constructor, or
back the array with a small enum-map type.

### 7. RNG / timing

`rng.rs` seeds xorshift from `tick as u32` and does `(next_u32() as usize) % (i + 1)`.
These are correct-by-inspection and low-risk. Low priority; could get
`usize::try_from().unwrap_or(0)` for hygiene but not worth churn.

## Recommended plan of attack (by ROI)

| Step | Action | Kills roughly |
|---|---|---|
| 1 | Add clippy lints to `engine/Cargo.toml` (`clippy::cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss` as **warn**) so new casts surface in CI | prevents regrowth |
| 2 | Introduce `CardId(i16)` newtype with `From<CardId> for usize` | ~250+ `as usize` |
| 3 | Widen modifier storage to `i32` / drop `u8` count fields | ~150 `as i16`, chunk of `as u8` |
| 4 | Centralize the clamp idiom in one helper | large share of `i32→u8` |
| 5 | Confine binary-decode casts to the decoder module with round-trip tests | most `as u16/u32` in blob code |
| 6 | Leave floats/RNG alone | — |

Steps 1–3 remove roughly half the cast sites and — more importantly — turn every
remaining conversion into something the compiler can reason about instead of a silent
truncation risk.

## Notes

- No `#![deny]`/`#![warn]` lint attributes exist anywhere in the crate;
  `src/lib.rs` only sets `recursion_limit` and a `no_std` cfg_attr. There's no
  `clippy.toml` and no `[lints]` section in `Cargo.toml`.
- Existing targeted suppressions are all `#[allow(clippy::too_many_arguments)]`
  (5 sites) — unrelated to casting.

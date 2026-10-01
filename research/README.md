# research/ — vendored third-party card-game AI

Reference implementations downloaded to read and copy ideas from. **Not built,
not imported, not on any crate's dependency path.** Nothing here compiles into
`rabuka_engine`; they are read as sources.

| repo | commit | why it is here |
|---|---|---|
| `gym-locm/` | `4a0c1c4` (2026-02-05) | The encoding + agents behind **ByteRL**, which won COG2022's double championship (`arXiv:2303.04096`). Source: https://github.com/ronaldosvieira/gym-locm |
| `SabberStone/` | `1db6e27` (2021-01-15) | Hearthstone simulator; `core-extensions/SabberStoneBasicAI` is a **search-based, no-neural-net** bot of the kind that actually placed in the Hearthstone AI Competitions. Source: https://github.com/HearthSim/SabberStone |

`.git` was stripped after checkout to keep the footprint down; the commit ids
above are the ones they were taken at.

## What was actually taken from them

### gym-locm → `engine/src/bot/encoding.rs`, `locm_card_features`

Ported `LOCMEnv.encode_card`: a **17-dim attribute vector per card** with card
ID deliberately excluded and **6 ability-keyword bits inside the vector**.

This repo's default encoding is the inverse — a 128-dim learned ID embedding
with no ability bits on cards at all, because `AbilityFeatures` is attached only
to on-stage positions. So **a card in hand carried no ability representation**,
and hand is precisely where the live set is chosen.

The keyword vocabulary is game-specific and was **mapped, not copied**; the
mapping is documented in the function. Covered by three tests:
`locm_encoding_gives_hand_cards_an_ability_signature`,
`locm_encoding_zero_fills_empty_slots`, `locm_encoding_separates_the_evaluated_deck`.

Reference state layout, for comparison with this repo's 2248 dims
(`gym_locm/envs/battle.py:48-66`):

```text
  3 x 2   player features
+ 8 x 17  cards in hand
+ 6 x  9  friendly cards on board
+ 6 x  8  enemy cards on board
= 528 dims  ->  shared_fc0 528->199  ->  {vf, pi, q} heads
```

`gym_locm/trained_models/` also holds **155 MB of trained weights** (180 JSON
files, `greedy` and `max-attack` families, 30k- and 1M-step horizons, per-seat
`1st`/`2nd`). These are LOCM-action-space weights and **cannot be used directly**
— different game, 145 discrete actions against this one's phase-dependent set.
They are kept because they are the only trained card-game policy weights
available under a permissive licence, and are the reference for the head shapes.

### SabberStone → not yet copied, but the reason it is here

`core-extensions/SabberStoneBasicAI/src/Score/` is a four-family evaluation
function set (`AggroScore`, `MidRangeScore`, `Control`, `Ramp`, `Fatigue`). The
relevant structural idea is that each `Rate()` is short and ordered:

```csharp
if (OpHeroHp < 1) return int.MaxValue;      // terminal, BEFORE any scoring
if (HeroHp  < 1) return int.MinValue;
if (OpBoardZone.Count == 0 && BoardZone.Count > 0) result += 5000;  // board PRESENCE
if (OpMinionTotHealthTaunt > 0) result += OpMinionTotHealthTaunt * -1000;
result += (BoardZone.Count - OpBoardZone.Count) * 5;               // discrete presence
result += MinionTotAtk;
result += (HeroHp - OpHeroHp) * 10;
```

Two transferable shapes, neither yet implemented:
1. **Terminal states short-circuit before any weighted term**, so a win can never
   be outvoted by a large positional term.
2. **Board presence is a discrete term**, not a gradient — an empty board against
   a non-empty one is a distinct, large event.

Point 2 is the closest external analogue to the one measurement this project
never converted into a term: §17.3 found the Main leaf's `coverage` (weight 60)
is a *threshold* over the whole requirement rather than a gradient, and §15.2-A
found the same leaf has no turn-awareness. Whether presence terms help here is
untested; Hearthstone's win condition is damage, this game's is placements, so
the transfer is a shape and not a weight.

## Licensing

- `gym-locm`: MIT (see its `LICENSE`).
- `SabberStone`: **GNU AGPL-3.0**. Vendored for reading only. Nothing from it has
  been copied into `engine/`; if any of it is ever ported, the AGPL's
  copyleft terms apply to the resulting work and that needs deciding first.

## Re-download

```powershell
git clone --depth 1 https://github.com/ronaldosvieira/gym-locm.git research/gym-locm
git clone --depth 1 https://github.com/HearthSim/SabberStone.git  research/SabberStone
Remove-Item research/*/.git -Recurse -Force   # strip history, ~80 MB each
```
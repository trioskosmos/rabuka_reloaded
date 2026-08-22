# Rules Gap Analysis

Diff of `engine\rules\rules.txt` (総合ルール ver. 1.06, 2026-04-28) against the current
engine implementation (`engine/src`). Confirmed-implemented rules are omitted.
Audited against source, `engine/tests/test_modules`, and card data
(`cards/cards.json`, `cards/abilities.json`) on 2026-08-22.

## Fully missing

### Section 7/8 — phase & turn-end triggers (`triggers.rs`)
| Rule | Description | Card data usage |
|---|---|---|
| **7.4.2** | ‘ターンの始めに’ / ‘アクティブフェイズの始めに’ / ‘ゲームの始めに’ triggers | 0 cards |
| **7.5.1 / 7.6.1 / 7.7.1** | phase-begin triggers (エネルギー/ドロー/メイン) + check timing before each | 0 cards |
| **8.2.1 / 8.4.1** | ライブカードセット/ライブ判定 フェイズの始めに | 0 cards |
| **8.4.10–8.4.12** | ‘ターンの終わりに’ auto triggers + the 8.4.9 → 8.4.11 stability loop (only resets/cleanup happen at LiveVictoryDetermination) | 0 cards |

No trigger text in this category exists in `cards/abilities.json`, so these are
currently theoretical; see the audit note in `src/triggers.rs`.

### Section 9 — continuous effect layering
| Rule | Description |
|---|---|
| **9.9.1.2–9.9.1.7** | Layered application order: grant/lose ability → non-numeric → set-value → additive → dependency ordering (9.9.1.6) → timestamp order (9.9.1.7/.1/.2). `recalculate_constants` applies typed modifier tables with no layering/timestamps. |

### Section 10/11/12 — rule processes & keywords
| Rule | Description | Note |
|---|---|---|
| **10.4 重複メンバー処理** | Duplicate members in one area: newest stays, others to owner's waitroom | Unreachable by construction today (baton-touch enforcement, swap-on-position-change, formation change forbids stacking); no defensive scan in `check_timing`. See comment at `turn/actions.rs::check_timing`. |
| **11.2.3** | Shared once-per-turn limit across `/` dual-keyword variants | Use limits keyed `(card_id, ability_index, turn)` (`core/game_state/mod.rs:120`); slash forms don't exist in card data yet. |
| **12.1 永久循環** | Loop negotiation procedure (active player declares loop + count, opponent accepts/reduces) | Only a crude `check_permanent_loop()` → draw guard exists. |
| **6.1.2** | Deck-construction replacement constant abilities (デッキ構築) | 0 occurrences in card data. |

### Section 1 — win/loss
| Rule | Description |
|---|---|
| **1.2.3 / 1.2.3.1** | Concede/resign: immediate loss bypassing check timing, immune to all card effects. No resign action exists anywhere. |
| **1.2.4** | Card effects that win/lose the game *mid-effect-resolution*; victory is only evaluated in `check_victory_condition`. |

## Partial

| Rule | Status |
|---|---|
| **6.1.1** | Deck legality is warn-only (`game/deck_builder.rs:84`): exact counts logged not enforced; max-4-copies-per-card-number has no check anywhere. |
| **3.1 / 4.1.7** | No owner-vs-master tracking; owner inferred from zone membership (`abilities.rs:553`, `move_cards.rs:1598`); controller≠owner handled ad-hoc in `state.rs:407/867`. |
| **4.1.2.3** | Hidden-zone guarantee: `blind` choice flag is prompt text only (`choice.rs:1267`) — player can't treat a matching hidden card as absent. |
| **4.1.4 / 4.1.4.1** | New-card-on-zone-change identity reset is ad-hoc (duration expiry instead of general rule). |
| **4.1.5** | Owner-chosen hidden ordering for simultaneous placements not modeled. |
| **5.6.3** | "Draw up to N" may-stop loop NOT implemented — `execute_draw_until_count` (`draw.rs:507`) auto-draws `target − current hand` with no per-card stop choice. |
| **5.8 入れ替える** | General zone-to-zone card swap (only member area-swap via PositionChange exists). |
| **1.3.2–1.3.3** | Maximize-satisfaction principle + prohibition-beats-instruction handled ad-hoc per site (e.g. Q118 guard `effects/mod.rs:49`), not systematic. |
| **8.4.11** | Exact expiry point of ターンの終わりまで／ライブ終了時まで durations vs. final check timing. |
| **9.3.2** | Invalidated effect parts skip their mandatory selections — unverified. |
| **9.3.4** | Default validity domains (member abilities on stage etc.) are implicit. |
| **9.6.3.1.2 / .1.3** | Can't-play-if-fixed-count-unselectable; zero-target → skip related effects — unverified. |
| **9.7.4.1.1–3** | Three-way visibility-based info snapshots for zone-move triggers. |
| **9.7.4.2** | Simultaneous entry counts as meeting own trigger condition. |
| **9.7.5 / 9.7.6** | General timed triggers (once-only) and re-arming state triggers ("手札にカードがない時" style). |
| **9.9.2 / 9.9.3** | Continuous effects dropping on stage-exit; zone-entry effects applying at entry instant. |
| **9.10.2** | Multiple replacements on one event: affected party chooses order (each-applies-once exists via `applied_this_event`). |
| **9.11** | Full last-known-information rules (partially via `RecentlyMoved`). |

## Verified implemented (previously suspected missing)

| Rule | Evidence |
|---|---|
| **2.3.2.1 ＆ name splitting** | `core/card.rs:491–504`; consumed by conditions/score/util. 63 ＆ names in card data. |
| **2.5 ユニット名 unit names** | `Card.unit` field + `same_unit_name` matching (`choice.rs:584–650`, `cost.rs:185`, group fallback `util.rs:507–520`). 2125 cards carry unit data. |
| **4.14 shared ResolutionZone** | Single global resolution zone on `GameState` (`mod.rs:89`), drained to active player at check timing (`actions.rs:1516–1525`). Matches rulebook. |
| **10.5 invalid-card processes**, refresh (10.2 incl. mid-effect deferral), victory (10.3), check-timing cascade (9.5.3), baton touch (9.6.2.3.2), Turn1/Turn2 (11.2/.3), position/formation change (11.10/11.11), center/left/right restrictions (11.7–11.9) | Covered by existing test suite (`cargo test --test run_all`). |

## Highest gameplay impact
1. Continuous-effect layering order (9.9) — verified missing by direct read of `recalculate_constants` (`modifiers.rs:221`): single additive pass, no set/add layers, no dependency or timestamp ordering.
2. End-of-turn triggers (8.4.10–12) — zero card usage today, but blocks future sets.
3. Phase-begin triggers (7.x/8.x) — same.
4. Mid-effect win/loss (1.2.4) — `check_victory_condition` only runs inside `check_timing`.
5. Permanent-loop negotiation (12.1) — `check_permanent_loop` (`abilities.rs:2515`) is hash-repeat → forced draw; no declaration procedure.
6. "Draw up to N" may-stop loop (5.6.3) — auto-draws without offering the stop choice.

## Stale docs fixed during audit
- `engine/tests/WRITING_TESTS.md`: removed claim that `check_timing` runs a duplicate-member safety check (no such code).

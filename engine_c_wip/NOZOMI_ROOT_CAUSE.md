# NOZOMI_ROOT_CAUSE.md — cluster N

**Agent:** nozomi (owner of `tests/test_parity_search_reorder.c`)
**Mandate:** the single biggest win — one root cause behind the ~42 FAIL assertions that
ranks 1–6 of the whole-tree top-10 all share.

> STATUS: measured and traced. **No engine file landed** — every file the fix needs is
> held by a live owner. All patches are written out below in landable form and were
> **built and A/B'd in a private snapshot** (`C:/Users/trios/AppData/Local/Temp/kilo/nozomi/`).

---

## 1. The signature, restated correctly

The brief says "the C opens a SelectCard where Rust opens a SelectTarget". That is the
*symptom*. The proximate cause is **not** a mis-dispatch in the choice renderer — it is
two independent gaps in the **effect → Choice PRODUCER** layer, plus a third, separate
defect that both of them unmask:

| # | defect | C site | Rust site it must match |
|---|---|---|---|
| **A** | `look_at` opens a Choice Rust never opens | `src/ability/effects/look.c:387-403` (`rb_effect_look_at`) | `engine/src/ability/look.rs:1055-1096` (`execute_look_at`) — fills `gs.looked_at_cards`, returns `Ok(())`, **never touches `gs.awaiting.choice`** |
| **B1** | no producer for `SelectTarget{target:"order"}` exists **at all** | absent; `RB_STK_ORDER` (`src/ability/enums.c:384`), the `"order"` answer arm (`src/ability/choice.c:3895`) and `rb_resolver_handle_order_selection` (`choice.c:3951`) are all unreachable | `engine/src/ability/effects/move/mod.rs:1855-1933` (`maybe_prompt_deck_order`), raised at `:2272` — **and** a second producer at `move/mod.rs:3139-3149` (the `look_and_select` `needs_order` arm) |
| **B2** | the order ANSWER handler exists but never places the cards | `src/ability/choice.c:3951-3972` — parses a comma-separated `"2,0,1"`, reorders the looked_at pool, then clears; **the cards are never put on the deck** | `engine/src/ability/choice.rs:2984-3055` (`handle_order_selection`) — removes ONE index per answer, re-arms the same prompt while >1 card remains, and on the last answer inserts every card at deck position 0 in reverse |
| **C** | *(pre-existing, not mine)* after a choice closes, the drain **re-executes the same ability entry from the top** | `src/ability/ability_queue.c:1403-1405` skip test vs the prompt-parking break at `:1584` | `engine/src/ability/ability_queue.rs:84` (`available_for`) + `resume_pending_actions` |

## 2. Proof, not theory

Direct probe in the snapshot (`rb_effect_look_at`, `rb_move_execute_move_cards_ported`):

```
[NZPROBE] rb_effect_look_at act=look_at src=deck_top dst=(null) cnt=3 pool=3      <- x13
[NZPROBE] execute_move_cards act=move_cards src=discard dst=stage cnt=1 all=(null) po=(null)
```

* `rb_effect_look_at` fires **13 times** and is the only producer of the prompt the test sees.
* `rb_move_execute_move_cards_ported` **never runs** for the ability's own second step
  (`src=looked_at dst=deck_top all=true placement_order=any_order`) — the sequential never
  gets past step 0, because step 0 parked the queue on a prompt Rust never raises.
* `grep '"order"'` over all of `engine_c_wip/src` returns exactly **three** hits, all of
  them *consumers*: the answer arm (`choice.c:3895`), the handler (`choice.c:3951`) and the
  enum strings (`enums.c`). **There is no producer.** Same for the Japanese prompt text
  `山札のカード順 — `Choose order for cards on deck`: **zero hits anywhere in the C.**

The card is `PL!-bp6-016-N` 東條 希, ライブ成功時「デッキの上からカードを3枚見る。それらを好きな順番でデッキの上に置く。」
Decoded (`cards/abilities.json`), a `sequential` of two steps and **no `custom`**:

```
actions[0]: { source:"deck_top", count:3, target:"self", action:"look_at" }
actions[1]: { source:"looked_at", destination:"deck_top", card_type:"card",
              all:true, placement_order:"any_order", action:"move_cards" }
```

So `misc.c:1053-1060` (`h_custom`) — the site the brief's "custom" hypothesis points at —
**does not apply to this card at all.** The effect is a plain two-step `sequential`.

## 3. Ranks 1–5, explained and measured

A/B in a private snapshot, both states built from the same tree with **only these hunks
reverted**. Objects deleted before each rebuild (the `Copy-Item`-preserves-`LastWriteTime`
trap makes a plain restore measure the OLD objects — it cost me one false "neutral" run
before I caught it).

| rank | assertion | count | before | after |
|---|---|---|---|---|
| 1 | `nozomi: expected a SelectTarget order prompt, got SelectCard` | 12 | 12 | **0** |
| 2 | `nozomi: the SelectTarget target is "order"` | 12 | 12 | **0** |
| 3 | `nozomi: the final remaining card needs no choice` | 6 | 6 | 6 (blocked on C) |
| 4 | `nozomi: nothing joins the hand (got 2 expected 0)` | 6 | 6 | **0** |
| 5 | `nozomi: nothing is discarded (all three return) (got 2 expected 0)` | 6 | 6 | **0** |
| — | `nozomi: the chosen top-three order AND the untouched suffix are exact` | 6 | 6 | 6 (re-worded) |
| — | `nozomi: the looked_at pool is emptied` | 0 | — | **+6 (new)** |

**36 of the 42 lines close. Suite total 122 → 92** (125 → 92 counting the two new invariant
cases added below). The `+6` are **not** a regression from the prompt work: they are defect C
— after the order answer, the ability re-fires (`[QUEUE_EXEC] … action=sequential`),
`look_at` runs a second time and steals three more cards off the deck.

Defect C is worth its own BLOCKED-ON. It is **pre-existing** (it fires in the untouched
baseline too, after the `look_at` answer), it is **not** caused by the re-entry latches
`g->just_completed_ability_key` (setting it changes nothing — measured), and it is not
`rb_queue_enqueue`/`rb_queue_push`/`rb_push_owned_entry` (all three are instrumented and print
**nothing** for the re-entry). It accounts for the residual on `parity_zone_source` rank 8
(`F: 真姫 exists exactly once (got 2 expected 1)` — the debut runs twice) and for 5 of the
`parity_restriction` lines I did not close (see §5).

---

## 4. The swept CLASS — "a parsed STRUCT FIELD read from the EXTRAS table"

`vm.c` `decode_effect_body` stores these keys in `AbilityEffect` struct fields and
`continue`s, so **they are NEVER in the extras table** and any `*_extra(e, "<key>")`
read of them is **permanently NULL**:

`text` · `action` · `source` · `destination` · `target` · `zone` · `count` · `condition`
· `card_type` (→ `card_type_field`) · `per_unit_count` · `cost_reduction_per_group`
· `actions`/`effect_steps`/`costs` (→ `child[]`) · `result_condition`
· `alternative_condition` · `look_action` · `select_action` · `gained_effect`

`vm.c:1091-1093` already documents this for `card_type`. `draw.c:444` and
`misc.c:292/455/515/540/730/877/1157/1311/1540`, `state.c:724/1487`,
`dynamic_count.c:656`, `score.c:524/823/996/1007`, `ability_effects.c:324/581`,
`card.c:959`, `look.c:969/1048`, `misc.c:…` already do the **field-first with an extras
fallback** idiom — that is the CORRECT shape. The **unguarded** instances are the class:

| file | lines | key |
|---|---|---|
| `src/ability/cost.c` | **353, 798, 897, 995, 1051, 1370, 1671, 2056, 2328, 2447** | `card_type` (10) |
| `src/ability/describe.c` | 336 / 360 / 369 / 375 | `count` / `source` / `destination` / `target` |
| `src/ability/util.c` | 2395 / 2446 / 2749 / 2755 | `card_type` / `per_unit_count` ×2 / `count` |
| `src/ability/compound.c` | 299 / 450 / 451 / 495 | `card_type` / `per_unit_count` ×2 / `source` |
| `src/ability/resolver.c` | 595 | `per_unit_count` |
| `src/core/card.c` | 756 / 865 | `destination` / `source` |
| `src/core/modifiers.c` | 1146 | `per_unit_count` |
| `src/ability/effects/draw.c` | **658 / 662** | `source` / `card_type` |
| `src/ability/effects/look.c` | 905 / 917 / 1119 | `card_type` |
| `src/ability/effects/misc.c` | 1185 / 1220 | `card_type` |
| `src/ability/effects/move.c` | 181 | `card_type` |
| `src/ability/effects/score.c` | 1031 | `card_type` |
| `src/ability/effects/state.c` | 124 / 1484 | `per_unit_count` |

**37 sites.** Two have measured FAIL impact and are in §6's patches
(`cost.c:1671` → −9 lines; `draw.c:658/662` → latent, the gates at `draw.c:665` and
`:670` happen to want exactly the NULL that the broken read produces).
`draw.c:658`/`:662` are **still unfixed in the live tree** — verified by reading the
current file, not assumed. The peer's report is correct and still open.

### Second member of the same class: a JSON ARRAY flattened to one string

`vm.c:1177-1186` (`decode_extra_value`) stores an effect **array** as ONE comma-joined
extras string, so `heart_colors:["heart03","heart04","heart05"]` arrives as
`"heart03,heart04,heart05"`. Rust reads `effect.heart_colors_any()` → a real
`Vec<String>` of 3. Every C reader that expects a LIST must split, or it collapses to
one element:

* `src/ability/effects/draw.c:170` `draw_heart_colors` — **measured**, §6 patch 4.
* `src/ability/effects/rules.c:1724` `r_heart_colors` — same bug, still latent
  (its only caller `rb_rules_execute_select_effect` at `rules.c:1580` has **zero callers**;
  the LIVE path is `draw.c`'s `rb_effect_select_effect`).
* Correctly-splitting readers for comparison: `misc.c:447/776` `split_heart_colors`,
  `executor.c:227`, `score.c:946` `sc_parse_colors`.

### Third member: triplicated handler families where the copy that RUNS is the incomplete one

* `rb_rules_execute_select_effect` (`rules.c:1580`, the faithful port of
  `selection.rs:27`, heart-colour arm included) — **zero callers**; the dispatched copy is
  `draw.c:653 rb_effect_select_effect`.
* `h_choose_target_player` (`misc.c:1040-1049`) — hardcodes `count=1` in
  `rb_emit_choice` and never walks `e->child`. Verified live, unchanged.
* `rules.c:1819`'s `changed_state_members` arm has zero callers while the live
  `misc.c:887 h_restriction` has no such arm at all. Same shape.

---

## 5. Ranks 6–10, attributed individually

| rank | suite | assertion | count | attribution | in my class? |
|---|---|---|---|---|---|
| 6 | `parity_restriction` | `colour choice must be offered` | 5 | **The array-flattening bug.** 栞子/嵐珠/かすみ/四季の 「appearanceのハートを1つ選び」 is `action:"select"`, `count:1`, `heart_colors:[…3]`, **no `source`**. The live dispatcher is `draw.c:653`, whose heart-colour arm is `draw.c:670` → `rb_effect_select_heart_color` (`draw.c:920`), whose `nu == 1` short-circuit (`draw.c:936`) fires because `draw_heart_colors` returns 1. Rust raises `Choice::SelectHeartColor` at `selection.rs:103-115`. **Closing it drops `parity_restriction` 42 → 18.** | **YES** |
| 7 | `parity_zone_source` | `F: the reveal is filtered to live cards` | 9 | **The extras-vs-field bug, in the REVEAL COST.** 真姫's debut cost is `{type:"reveal", source:"hand", zone:"hand", count:1, card_type:"live_card", optional:true}` — a **cost**, not an effect. `cost.c:1671` reads `card_type` from the extras table (permanently NULL), so the reveal pool at `cost.c:1694-1717` matches every hand card and `pending->card_type` at `cost.c:1759` is never set. Note `cost.c:1669` reads `source` from the **struct field** correctly, one line above — the exact contrast. **Closing it drops `parity_zone_source` 51 → 42.** | **YES** |
| 8 | `parity_zone_source` | `F: CROSSROADS is in the waitroom` | 5 (+1 sibling) | **NOT downstream of rank 7** — verified: rank 7 closes, rank 8 does not move. Two contributors: (a) defect C above, evidenced by the sibling `F: 真姫 exists exactly once (got 2 expected 1)` — the debut executes **twice**, so 真姫 lands twice; (b) the placement leg, `move_cards` with `source:"success_live_zone"` / `source:"selected_cards"` (`move_missing.c`), which I did **not** isolate. | partly |
| 9 | `parity_wave1c` | `B1 COST LEG 1 NOT PAID: the 起動 costs 2E … half-paid activation` | 5 | **NOT my class — proven.** `PL!HS-bp1-002-RM` 村野さやか 起動 has `cost:{type:"sequential_cost", costs:[{type:"pay_energy",energy:2,count:2},{source:"stage",destination:"discard",card_type:"member_card",self_cost:true,type:"move_cards"}]}`. `costs` decodes into `child[]` (`vm.c:859`) and `cost.c:1478-1531` dispatches each child through `pay_cost_inner` (which handles `move_cards` at `cost.c:1566`), so the machinery exists — yet neither leg runs and the effect does. **T6-6 did NOT cover it:** T6-6 is the mandatory-cost *refusal* in `rb_activate_card` (`actions/mod.rs:553-564`), i.e. the opposite failure (refuse when unpayable); this is "payable, never paid". Still 5/5 red after T6 landed. Measured neutrality: my 5 patches change `parity_wave1c` by **exactly 0 lines** (identical FAIL multiset). Owner: `cost.c` (B) / `game_state_abilities.c`. | **no** |
| 10 | `parity_hearts_live_start` | `second selection prompt should appear` | 4 | **NOT my class — proven** (4 → 4, byte-identical multiset under all 5 patches). 姫野 `PL!HS-bp5-006-R` 「手札の同じグループのカードを2枚選ぶ」 is a **count-2 hand cost**. The C opens ONE `SelectCard` and the answer path never re-prompts for the remaining count, so the second Rust pick has no prompt to answer. Rust `select_indices(&[i])` on a count-2 choice re-arms. Sibling assertions (`only the 蓮ノ空 pair (indices 0,1) is selectable`, `both same-group cards go to the waitroom`) pass, so the FILTER is right and only the **multi-pick re-prompt** is missing. Owner: `cost.c` emitter + `choice.c` answer path. | **no** |

---

## 6. The patches — all built and A/B'd, none landed

Snapshot: `C:/Users/trios/AppData/Local/Temp/kilo/nozomi/engine_c_wip`, re-snapshotted
after T3's header bill so both states build against the current `include/rabuka.h`
(`sizeof(GameState)` grew). Baseline = the same snapshot with **only these five files**
reverted; every rebuild deleted every `.o` first, because `Copy-Item` preserves
`LastWriteTime` and a plain restore silently measures the OLD objects — that trap cost me
one byte-identical false "neutral" reading before I caught it.

### Definite A/B, 24 suites, both states fully rebuilt

| suite | PRE | POST | Δ |
|---|---|---|---|
| `parity_search_reorder` | 122 | **89** | **−33** |
| `parity_restriction` | 42 | **18** | **−24** |
| `parity_sequential` | 112 | **99** | **−13** |
| `parity_zone_source` | 51 | **42** | **−9** |
| `parity_jidou_watch` | 55 | **52** | **−3** |
| `parity_cce_gaps` | 54 | **52** | **−2** |
| `parity_hearts_live_start` | 63 | 64 | **+1** |
| `parity_wave1c` | 31 | 31 | 0 (identical multiset) |
| `parity_jidou_watch_clusters` / `parity_jidou_move_yell` / `parity_draw_flat` / `parity_small_clusters` / `parity_integration` / `parity_under_member` / `parity_zones_position` / `parity_live_total` / `parity_recover_live` / `parity_wave1a` / `parity_wave1b` | — | — | 0 |
| `parity_full_round` / `parity_full_round2` / `parity_full_round3` / `restriction_mechanics` / `mechanics` | 0 | 0 | 0 (all produced real output — byte counts 7 k–86 k, so they RAN; not the hidden-failure trap) |
| **TOTAL** | **1022** | **939** | **−83** |

The single regression is `parity_hearts_live_start`: one line,
`area-moved member gains the chosen heart until live end (got 2 expected 1)` — a
redistribution inside an already-failing test (its sibling assertions shifted), net +1.

### BLOCKED-ON 1 — `src/ability/effects/look.c:387-403` (cluster H / T2 owner)

Delete the spurious prompt. Pool fill and origin recording already happen in
`look_at_fetch` → `look_pool_reset` (`look.c:273`), which is the C's
`carried.looked_at_origin`; only the `rb_emit_choice` + park block goes.

```c
void rb_effect_look_at(GameState *g, int actor, AbilityEffect *e)
{
    int who = look_target_player(g, actor, e);
    int count = look_at_fetch(g, actor, e);
    if (count <= 0) return;
    /* look.rs:1055-1096 — execute_look_at FILLS the pool and returns Ok(());
       it opens NO Choice. 見る informs (rule 5.7). The prompt the player answers
       belongs to whatever CONSUMES the pool — for 「3枚見て好きな順番でデッキの
       上に置く」 that is the deck-order SelectTarget, never a card pick over the
       freshly revealed pool. */
    look_push_look_log(g, count);
    g->queue.resume_look_owner = who;
    g_look_pending_select = NULL;
    g_look_pool_select_offered = 0;
}
```
(deletes the `rb_emit_choice(... RB_CHOICE_SELECT_CARD, "looked_at" ...)`,
`rb_queue_pause_for_choice`, `resume_mode = 2`, `resume_eff`, `resume_is_select = 0`,
`resume_actor`, `resume_host` lines that follow `if (count <= 0) return;`)

**Risk measured: LOW.** `parity_look_reveal` 1→2, `parity_blades_live_start` 34→35 and
`parity_mill_deck` 52→54 are the only look-touching regressions, all redistributions
inside already-red tests; `parity_look_and_filter` improves 27→21 and `parity_jidou_watch`
55→52. The `static TestGame` rule is respected throughout — no new non-static locals.

### BLOCKED-ON 2 — `src/ability/effects/move_missing.c` (NOT in the ownership map — I claim it)

`move_missing.c` is **unclaimed**: it appears in neither the OWNERSHIP MAP nor the brief's
held list, and `make_t_parity_search_reorder` is byte-identical to the live copy both
before and after my hunk, so nothing collides.

Add the producer (Rust `move/mod.rs:1855-1933`, called at `:2272`) — insert immediately
after the `if (e->is_optional) { … return; }` block and before the distinct filter:

```c
/* move/mod.rs:1855-1933 — DeckOrderHint. Returns 1 when the prompt was issued. */
static int s_order_cards[RB_MAX_ZONE];
static int s_order_n;
static int s_order_player;
int rb_move_handle_order_selection(GameState *g, const char *selected);

static void cmf_emit_order_prompt(GameState *g, int actor, int pl, int n_pool)
{
    char desc[128];
    snprintf(desc, sizeof(desc), "Choose order for cards on deck (%d cards)", n_pool);
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, n_pool, 0, "order");
    rb_choice_set_description(&g->queue.pending, desc);
    rb_queue_pause_for_choice(g, &g->queue.pending);
    g->queue.resume_mode = 7;
    g->queue.resume_is_select = 0;
    g->queue.resume_look_owner = pl;
}

/* move/mod.rs:1855-1933.  Returns 1 when the prompt was issued (caller returns). */
static int cmf_maybe_prompt_deck_order(GameState *g, int actor, const AbilityEffect *e,
                                       const int *taken, int n_taken,
                                       const char *source, const char *destination)
{
    if (!g || !taken || n_taken <= 1) return 0;
    int is_deck_dest = !strcmp(destination, "deck") || !strcmp(destination, "deck_top") ||
                       !strcmp(destination, "deck_bottom");
    if (!is_deck_dest) return 0;
    int is_eligible_source = !strcmp(source, "discard") ||
                             !strcmp(source, "selected_cards") ||
                             !strcmp(source, "hand") ||
                             (!strcmp(source, "looked_at") && extra_true(e, "all"));
    if (!is_eligible_source) return 0;
    const char *po = cmf_extra(e, "placement_order");
    if (!po || strcmp(po, "any_order")) return 0;

    int pl = actor;
    if (e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    if (pl < 0 || pl > 1) pl = actor;
    int pool[RB_MAX_ZONE];
    for (int i = 0; i < n_taken && i < RB_MAX_ZONE; i++) pool[i] = taken[i];
    /* mod.rs:1905-1914 — order_pool = taken, REVERSED for a Hand source. */
    if (!strcmp(source, "hand"))
        for (int i = 0, j = n_taken - 1; i < j; i++, j--) {
            int t = pool[i]; pool[i] = pool[j]; pool[j] = t;
        }
    rb_look_clear(pl);
    for (int i = 0; i < n_taken; i++) rb_look_add(pl, pool[i]);
    s_order_n = 0;
    s_order_player = pl;
    cmf_emit_order_prompt(g, actor, pl, n_taken);
    g->queue.resume_eff = (AbilityEffect *)e;
    g->queue.resume_actor = actor;
    return 1;
}
```

and the call site:

```c
    /* move/mod.rs:2272-2281 — issued HERE: after the source resolve and the
       optional/armed bookkeeping, BEFORE the distinct filter and
       `place_taken_cards`. Rust returns early when it fires. */
    if (cmf_maybe_prompt_deck_order(g, actor, e, ids, n, source, destination))
        return;
```

**Do NOT land this without BLOCKED-ON 3.** With today's `choice.c` the prompt would be
answered by a handler that reorders the pool and never places the cards — strictly worse.

### BLOCKED-ON 3 — `src/ability/choice.c:3951-3972` (cluster B owner)

Replace the body of `rb_resolver_handle_order_selection` (the call site at
`choice.c:3895` needs no change):

```c
/* handle_order_selection — choice.rs:2984-3055. The old body parsed a
 * comma-separated "2,0,1", reordered the looked_at pool by those indices and
 * then cleared the choice state — it never PLACED the cards, so 「好きな順番で
 * デッキの上に置く」 left them on the table. Rust removes ONE index per answer,
 * re-arms the same SelectTarget while more than one card remains, and on the
 * last answer drains the remainder and inserts every card at deck position 0 in
 * reverse. */
int rb_move_handle_order_selection(GameState *g, const char *selected);

/* Returns 0 when the order is complete and the caller must clear the choice state
 * and resume; -1 on an invalid answer (Rust's Err); 1 when it re-armed. */
int rb_move_handle_order_selection(GameState *g, const char *selected)
{
    if (!g || !selected) return -1;
    int pl = s_order_player;                     /* declared in move_missing.c */
    if (pl < 0 || pl > 1)
        pl = (g->queue.actor >= 0 && g->queue.actor <= 1) ? g->queue.actor : 0;
    int pool[RB_MAX_ZONE];
    int n = rb_looked_at_pool(pl, pool, RB_MAX_ZONE);
    char *end = NULL;
    long idx = strtol(selected, &end, 10);       /* choice.rs:2994-2998 */
    if (!end || *end || end == selected || idx < 0 || idx >= n) return -1;
    if (s_order_n < RB_MAX_ZONE) s_order_cards[s_order_n++] = pool[idx];
    rb_look_remove(pl, pool[idx]);
    n = rb_looked_at_pool(pl, pool, RB_MAX_ZONE);
    if (n > 1) {                                  /* choice.rs:3004-3020 */
        s_order_player = pl;
        cmf_emit_order_prompt(g, g->queue.actor >= 0 && g->queue.actor <= 1
                                  ? g->queue.actor : pl, pl, n);
        return 1;
    }
    for (int i = 0; i < n && s_order_n < RB_MAX_ZONE; i++)
        s_order_cards[s_order_n++] = pool[i];
    rb_look_clear(pl);
    RbPlayer *P = &g->p[pl];                      /* choice.rs:3041-3044 */
    for (int i = s_order_n - 1; i >= 0; i--) {
        if (P->deck.n >= RB_MAX_ZONE) break;
        for (int k = P->deck.n; k >= 1; k--) P->deck.cards[k] = P->deck.cards[k - 1];
        P->deck.cards[0] = s_order_cards[i];
        P->deck.n++;
    }
    s_order_n = 0;
    return 0;
}

void rb_resolver_handle_order_selection(GameState *g, const char *selected) {
    if (!g) return;
    int r = rb_move_handle_order_selection(g, selected);
    if (r < 0) return;                 /* Err: the prompt stands (choice.rs:2994-2998) */
    RbAbilityResolver tmp; memset(&tmp, 0, sizeof(tmp)); tmp.gs = g;
    if (r == 0) { rb_resolver_clear_choice_state_and_resume(&tmp); return; }
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;   /* the handler re-armed itself */
}
```

> Cleaner variant, if cluster B prefers to own the state: move `s_order_cards` /
> `s_order_n` / `s_order_player` into `move_missing.c` (they are the C's
> `LookAndSelectStep::Order { ordered }` context, file-static for the same reason
> `g_look` is) and export only `rb_move_handle_order_selection` — that is what I built.

### BLOCKED-ON 4 — `src/ability/effects/draw.c:170` and `:658`/`:662` (cluster F owner)

**(a) rank 6's fix** — `draw_heart_colors` must split the comma-joined array:

```c
/* Collect all heart_colors extra values.
 *
 * vm.c:1177-1186 stores an effect ARRAY as ONE comma-joined extras string
 * (`decode_extra_value`), so `["heart03","heart04","heart05"]` arrives as
 * "heart03,heart04,heart05". Rust reads `effect.heart_colors_any()` — a real
 * `Vec<String>` of three — so the reader must SPLIT. Without the split the list
 * collapses to one element, `rb_effect_select_heart_color`'s `nu == 1`
 * short-circuit (draw.c:936) fires, and the prompt Rust raises at
 * selection.rs:103-115 is never raised. */
static int draw_heart_colors(const AbilityEffect *e, const char **out, int max) {
    int n = 0;
    if (!e) return 0;
    for (int i = 0; i < e->n_extra && n < max; i++) {
        if (!e->extra_k[i] || strcmp(e->extra_k[i], "heart_colors") || !e->extra_v[i])
            continue;
        const char *cursor = e->extra_v[i];
        char buf[256];
        size_t used = 0;
        for (;;) {
            const char *comma = strchr(cursor, ',');
            size_t len = comma ? (size_t)(comma - cursor) : strlen(cursor);
            if (len == 0) { if (!comma) break; cursor = comma + 1; continue; }
            if (used + len + 2 >= sizeof(buf)) break;
            if (used) buf[used++] = ',';
            memcpy(buf + used, cursor, len);
            used += len;
            buf[used] = '\0';
            static char store[8][192];
            int slot = n % 8;
            snprintf(store[slot], sizeof(store[slot]), "%s", buf);
            out[n++] = store[slot];
            if (!comma) break;
            cursor = comma + 1;
        }
    }
    return n;
}
```

**(b) the peer's report, still open** — field-first for `source`/`card_type`:

```c
    const char *src   = (e->source && e->source[0]) ? e->source : draw_extra(e, "source");
    /* ... or_ct / chars / group unchanged ... */
    const char *ctype = e->card_type_field[0] ? e->card_type_field
                                               : draw_extra(e, "card_type");
```

Latent (0 measured FAIL delta in 24 suites) but it is the same defect the peer reported
and `draw.c:444` already has the correct idiom. Note the gates at `draw.c:665`/`:670`
read as Rust's `source.is_none() && card_type.is_none()`, so this is safe to land with (a).

### BLOCKED-ON 5 — `src/ability/cost.c:1671` (cluster B owner) — **rank 7, −9 lines**

```c
     if (cost_is_reveal(cost)) {
         const char *source = cost->source ? cost->source : "hand";
         const char *target = cost->target ? cost->target : "self";
+        /* vm.c:1093-1108 stores `card_type` in the STRUCT FIELD and `continue`s,
+           so it is NEVER in the extras table: this read is unconditionally NULL
+           and the reveal pool below is unfiltered. Rust reads the parsed
+           `effect.card_type_any()`. Same idiom, already right at draw.c:444. */
+        const char *card_type = cost->card_type_field[0] ? cost->card_type_field
+                                                        : eff_extra(cost, "card_type");
```

⚠️ `cost.c` has **ten** `eff_extra(cost,"card_type")` reads at 353, 798, 897, 995, 1051,
1370, 1671, 2056, 2328, 2447 with the **same text**. The `edit` tool silently patched the
WRONG one (798) when I gave it that string alone — anchor on the surrounding
`if (cost_is_reveal(cost)) {` block, as above, and re-verify by line number.

### BLOCKED-ON 6 — the re-fire (cluster B / B1 owner) — **worth more than anything above**

After a choice closes, `rb_drain_ability_queue` executes the **same entry again from the
top**. Measured, with unconditional instrumentation added at the drain's own skip test:

```
[NZSCAN] idx=0 started=0 completed=0 pend=0
[QUEUE_EXEC] idx=0 n=1 cid=2639 … action=sequential
```

i.e. the entry reaches `ability_queue.c:1403-1405` with `effect_started == 0`. The
prompt-parking break at `ability_queue.c:1584` is reached before anything stamps the entry.
Consequences: 6 residual `nozomi` lines + **6 NEW** `nozomi: the looked_at pool is emptied
(got 3 expected 0)` (the re-run `look_at` steals three more cards off the deck), rank 8's
`真姫 exists exactly once (got 2 expected 1)`, and 5 `parity_restriction` lines whose chosen
heart is overwritten by the re-raised prompt answered with option 0.

Ruled out by measurement, not inference:
* `g->just_completed_ability_key` — stamping it in the answer path changes **nothing**
  (identical counts; and `triggers.c:566`'s veto is cleared anyway by
  `game_state_abilities.c:2393`).
* `rb_queue_enqueue` / `rb_queue_push` / `rb_push_owned_entry` — all three instrumented,
  **zero prints** for the re-entry, so the entry is not re-enqueued through any of them.
* `rb_drain_seen_add`'s per-drain dedup (`ability_queue.c:1387-1395`) — only covers one
  drain.

Rust's counterpart is `queue.rs:84` `available_for` + `resume_pending_actions`, which never
re-runs a completed entry. I did not land a speculative fix: the correct latch site belongs
to the queue-resume owner and B1 is already contested.

---

## 7. Tests landed (the only file I edited)

`tests/test_parity_search_reorder.c` — two new cases, registered as
`nz/look_at_verb_opens_no_choice` and `nz/deck_order_prompt_is_select_target_order`.
Both are **red by design in the live tree** (3 `^FAIL`) and **green with the patches** —
they pin the INVARIANT rather than the ability, so a refactor cannot re-open it behind a
still-red suite. Neither weakens or removes any existing assertion; both `static`
`TestGame` locals; both use `test_new_id`, not `test_id`.

* `nz(1)` calls `rb_effect_look_at` directly with a Rust-shaped 「3枚見る」 and asserts
  `looked_at_cards` is filled to 3 **and** that no Choice is pending, citing
  `look.rs:1055-1096`.
* `nz(2)` fires the real ライブ成功時 and asserts the first prompt is a
  `SelectTarget` with `target == "order"` and `allow_skip == false`, citing
  `move/mod.rs:1915-1932` / `:1926-1931` / `:1929`.

Ranks 6–10 need no new tests: every one of them already has a red assertion in the owning
suite, written from the Rust test.

---

## 8. Files touched

| file | what |
|---|---|
| `engine_c_wip/tests/test_parity_search_reorder.c` | **only live edit** — 2 invariant cases |
| `engine_c_wip/NOZOMI_ROOT_CAUSE.md` | this file (new) |
| `engine_c_wip/PARITY_CHECKLIST.md` | cluster **N** |
| `look.c`, `move_missing.c`, `choice.c`, `draw.c`, `cost.c` | **snapshot only, never the live tree** |

No git writes. `HEAD` is still `f152a50f`. No `.o` in the live tree.
`src/abilities_strings.bin` verified 726831 B / `89204df0bbde08a0…` before and after
every build; `make data` was never run.
---

# 9. Follow-up pass — cluster X (decode key read from the wrong table)

Everything in §1-§8 above was measured before the four free patches were
landed. This section records what changed, what the patches actually cost, and
the three findings that correct §6 above.

## 9.1 Measurement basis

Suites are the 33 names in circulation **plus 8 real substitutes for the 5
that do not exist**, 41 in total:

```
parity_search_reorder parity_zone_source parity_restriction parity_wave1c
parity_hearts_live_start parity_jidou_watch parity_jidou_watch_clusters
parity_jidou_move_yell parity_draw_flat parity_sequential parity_small_clusters
parity_integration parity_cce_gaps parity_under_member parity_zones_position
parity_live_total parity_live_total2 parity_recover_live parity_full_round
parity_full_round2 parity_full_round3 restriction_mechanics mechanics
parity_wave1a parity_wave1b parity_executor_dispatch audit_fields audit_zones
vm_decode parity_condition_and_effect parity_t7_condition_decoder
parity_look_and_filter parity_look_reveal parity_blades_live_start
parity_mill_deck parity_target_player parity_cost_discount
parity_conditional_gated parity_ability_watch parity_jidou_state
parity_recover_score_state_delta
```

**Five of the 33 briefed suite names DO NOT EXIST** (re-confirmed with
`Test-Path`): `parity_card_filters`, `parity_effect_conditions`, `parity_vm`,
`parity_resolver`, `parity_condition_decode`. Their real substitutes are in the
list above. A run over the briefed 33 alone would have reported 28 suites and
missed `parity_executor_dispatch` and `parity_look_and_filter` entirely — both of
which this work moved.

Every A/B deleted the affected `.o` before rebuilding, and the final
measurement deleted **all 42** and did a full `make all`. No byte-identical
before/after was accepted as evidence.

| snapshot | total `^FAIL` (out + err) |
|---|---|
| baseline (only this agent's hunks reverted) | **2137** |
| after §9.2 + §9.3 | **2117** |

## 9.2 PATCH 1 LANDED — `look.c` `rb_effect_look_at`

The spurious `SelectCard{zone:"looked_at", count}` + queue-park block is gone;
`rb_effect_look_at` now only fills the pool (already done by
`look_at_fetch` → `look_pool_reset`, which is the C's
`carried.looked_at_origin`), records `resume_look_owner`, and pushes the 確認
log.

**Pre-flight census (`tools/audit_x_lookat.c`, through the engine's own
decoder):** `look_at` nodes = **200**, carrying `optional` = **0**, all 200
carrying a `source`, and **15** are step 0 of a `sequential`. So the only
prompt Rust's `execute_look_at` can raise — `maybe_offer_look_cost_gate`
(`look.rs:995-1013`), an OptionalCost skip gate — has **no corpus user**, and 15
sequential chains were blocked at step 0.

A/B, 41 suites:

```
baseline -> patch 1 + test fixes      total 2137 -> 2117   (-20)
  parity_cce_gaps            94 -> 90   (-4)
  parity_look_and_filter     45 -> 39   (-6)
  parity_search_reorder     158 -> 148  (-10)
  no other suite moved
```

**Zero regressions.** Two suites needed their assertions corrected first,
because they pinned the buggy behaviour rather than Rust's:

* `tests/test_parity_executor_dispatch.c` — `look_at prompts over the looked-at
  pool for its own count` and `look_at's choice is scoped to the looked-at zone`
  asserted the C-only `SelectCard`. The comment above them said so outright:
  *"A bare look_at raises its own SelectCard choice over the looked-at pool
  (look.c rb_effect_look_at)"* — a statement about the C, not about Rust. They
  now assert what `look.rs:1055-1096` does: **no choice is opened**. The three
  `rb_resume_with_choice(-1)` calls in that file were reaching a prompt that no
  longer exists, so they now call `rb_look_resume_indices(..., 0, NULL, ...)` —
  the engine's own rule-5.7 *declined-consumer* path, which is the only thing
  that ever returns a looked-at pool to its origin.
* `tests/test_parity_look_reveal.c` — `test_look_at_counts_and_zones` did the
  same, and its `all=true` sub-case was silently measuring a deck the previous
  sub-case had left short. Same repair.

Per-assertion, in `parity_search_reorder`, **closed** by patch 1:

```
- x12  expected a SelectTarget order prompt, got SelectCard
- x12  the SelectTarget target is "order"
-  x6  nothing joins the hand (got 2 expected 0)
-  x6  nothing is discarded (all three return) (got 2 expected 0)
-  x1  nz(1): look.rs:1055-1096 opens NO choice      <- nozomi's own invariant
```
plus 2 test verdicts in `parity_cce_gaps` (`j2_one_member_discard`,
`j4_three_skip_all`) and 4 assertions there.

**Opened** by patch 1, and this is the honest cost:

```
+ x6   nozomi: the mandatory order prompt is open
+ x12  vacuity control: nozomi: the mandatory reorder must prompt at least once
+  x2  nz(2): move/mod.rs:1915-1932 opens the mandatory deck-order prompt
+  x1  nz(2): move/mod.rs:1929 allow_skip is false
+  x1  FAILED rt/live_success_not_on_debut   (was 9 checks / 0 failures)
+  x2  kaho: the 7th pass must raise the deck-top inspection (nothing pending)
```

All of those are the **same single cause**: with the wrong prompt deleted and
the right one still missing, the 見る verb raises nothing at all until patch 2
lands. They are the vacuity controls for exactly the producer that
`choice.c:3951-3972` blocks, and they go green the moment patches 2 + 3 land
together. `rt/live_success_not_on_debut` is the one test verdict patch 1 costs;
its own headline assertion (`must not fire on debut`) still passes — only its
positive-sibling block moved.

## 9.3 PATCH 2 NOT LANDED, PATCH 3 NOT PREPARED — hard ordering

`move_missing.c` is unclaimed and mine, so patch 2 (`maybe_prompt_deck_order`,
`move/mod.rs:1915-1932` called at `:2272`) was written out — and **not
landed**, exactly as instructed, because `choice.c`'s answer handler
(`choice.c:3951-3972`) parses `"2,0,1"`, reorders the pool, and **never places
the cards on the deck**. Landing the producer alone would replace a wrong prompt
with a prompt whose answer does nothing. Reported as `BLOCKED-ON: choice.c`
with the patch text.

## 9.4 Cluster X — the class itself, swept

The four confirmed sites in my files are fixed (field-first, `look.c`'s own
`look_filter_card_type` reused where it existed):

| site | key | Rust twin |
|---|---|---|
| `look.c:919` `rb_effect_reveal_until_chosen_card` | `card_type` | `look.rs:1384` `effect.card_type_any()` |
| `look.c:933` `rb_effect_reveal_until_target` | `card_type` | `look.rs:1344` |
| `resolver.c:602` `apply_per_unit_cost_reduction` | `per_unit_count` | `resolver.rs:1497` `per_unit_count_any()` |
| `score.c:1038` `rb_execute_modify_required_hearts_success` | `card_type` | `score.rs:792-853` |

Nine further reads in my files were audited and are already correct
(`look.c:976/1053/1129`, `state.c:124/399/724/1484/1487`,
`ability_effects.c:324/581`, `phase.c:147`) — see
`PARITY_CHECKLIST.md` cluster X for the row per site.

**The measured headline that contradicts §4's framing:** all four are
**corpus-inert**. Reachability, through the engine's own decoder:

```
reveal_until_chosen_card            0 nodes
reveal_until_target                 0 nodes
modify_required_hearts_success      0 nodes
modify_cost + per_unit + subtract   7 nodes, ALL 7 carry per_unit_count == 1
```

The last row is why the sweep moved no FAIL count: the broken reader returned
`per_unit_count = 1`, and 1 is the value all 7 nodes carry. Rust's
`per_unit_count_any().unwrap_or(1)` returns the same 1. **The class fixes were
worth 0 lines across 41 suites, and that is exactly what the census predicted
before the A/B was run.**

They are landed anyway, because the readers were wrong, and
`tests/test_parity_x_class.c` (new, 5 cases, 20 checks) pins all four:
against the reverted readers it reports 3 of 5 cases FAILED
(`score` positive control → 0 modifier, i.e. a total silent no-op;
`reveal_until_chosen_card` pool 3 instead of 2; `reveal_until_target` pool 0
instead of 2), and 5 of 5 ok against the fixed ones.

## 9.5 §6 patch 6 REFUTED — the stated site is not the defect

§6 patch 6 asked to fix `ability_queue.c:1403-1405`. Instrumenting the skip
test at `:1378` (`if (e->effect_started || e->completed) continue;`) with
`[NZSCAN]`, and every entry builder with `[NZENQ]`, gives:

```
[NZSCAN] idx=0 n=1 cid=2639 ab=0 started=0 completed=0 cur=0
```

`started=0` and `n_entries == 1` on **every** re-visit. So the drain is **not**
re-running a stamped entry — the entry is **fresh**. A fresh entry means a new
one was built, and the builders print:

* ライブ成功時 / 常時 shapes: `rb_build_ability_queue_entry`
  (`game_state_abilities.c:1673`, `memset` then `effect_started` left 0).
* `PL!N-bp1-002-R`: `rb_queue_enqueue` via `rb_push_owned_entry`
  (`triggers.c:96`), and `[NZPUSH]` shows the trigger is **DEBUT**, not
  ライブ成功時 — so it comes from `rb_trigger_debut` (`:268`), reached from
  `rb_move_fire_debut_side_effects` (`move.c:2141`, cluster F), which is called
  **per moved card** at `move.c:2135`.

**A/B of the instrumentation stream: byte-identical with and without patch 1**
(15 `[NZSCAN]`, 14 `[QUEUE_EXEC]` both ways). The duplicate-`looked_at` report is
**pre-existing** and patch 1 neither causes nor cures it.

So §6 patch 6's premise ("the drain re-executes the same entry from the top")
does not hold, and a latch at `:1403-1405` would be fixing a site that is
already correct. Two further corrections to §4/§5, both from my own
measurement errors:

* **`rb_effect_reveal_until_target` is NOT callerless and has a Rust twin.** It
  is dispatched from `triggers.c:1197` and `misc.c:994` and exercised by
  `tests/test_parity_look_reveal.c:505/527`. Its twin is
  `look.rs:1328-1369`. My first pass missed both because I searched with a
  `src\**\*.rs` glob that does not recurse under PowerShell.
* **`const char *puc = ...` in `resolver.c` reads the FIELD first**, so the
  extras read was a dead fallback, not the defect. §4's inclusion of
  `resolver.c:595` as an unguarded site was wrong; the real defect there was the
  missing field read, which is now fixed.

### `rules.c:1580` — the duplicate-entry-point question, answered

**No, `rb_rules_execute_select_effect` must not become the dispatched entry
point.** `rules.c:1724`'s `r_heart_colors` is the same array-flattening defect
that `draw.c:170` had (`heart_colors:[a,b,c]` arriving as `"a,b,c"`), so the
two implementations are the same hazard twice. But swapping the dispatch would
be wrong for a different reason:

* `rb_rules_execute_select_effect` is **0-caller, while `draw.c:653`'s copy is
  live** — the shape of `execute_gain_ability_from_source`, which a peer found
  implemented twice with both copies live and diverging on four points.
  Promoting the dead copy would be a straight **behaviour change on every
  `select` effect**, not a de-duplication, and it would land untested because
  the corpus census cannot tell the two apart.
* `PARITY_CHECKLIST.md` cluster C already records `rules.c` as the C-holdout's
  file, and cluster C is where a `select_effect` parity gap would belong.
* The correct order is: fix the array-flattening defect at whichever site is
  **live** first (`draw.c:653`, cluster F), prove the fix, and only then delete
  `rules.c:1580` as dead. Merging them the other way round makes the live
  behaviour depend on the untested copy.

I did not touch `rules.c`.

## 9.6 Found, NOT fixed

* **`triggers.c:1189-1197` is a second dispatcher.** `apply_constant_effect`
  routes `look_at`, `reveal` and `reveal_per_group` all onto
  `rb_effect_look_at`, and adds a `reveal_until_target` arm that Rust's
  `dispatch.rs` has no `ActionType` for. Rust sends those three to three
  different functions (`dispatch.rs:78-83`). Measured with the engine's own
  classifier (`tools/audit_x_const.c`): 275 常時 abilities, 16 distinct actions,
  and **none** of them is `look_at`, `reveal`, `reveal_per_group`,
  `reveal_until_live_card`, `reveal_until_chosen_card` or `reveal_until_target`.
  **Latent, 0 of 275 — VERIFIED-NOOP, no fix needed today.** If a future card
  adds a 常時 `reveal`, the C would route it to `execute_look_at` and open a
  pool Rust never fills.
* **`src/test_game.o` is in the Makefile's `OBJ_LIB` but no `SRC` entry,**
  so `make all` never builds it and 8 of the 41 suites fail to LINK against a
  clean tree. `make t T=...` happens to build it via the `src/%.o: src/%.c`
  pattern rule; `make all` does not. Any agent who measures with a hand-rolled
  gcc invocation must build it explicitly first, or it silently drops suites.
* **`misc.c:1040-1049 h_choose_target_player` hardcodes `count=1` and never
  walks `e->child`** — confirmed, `misc.c` is cluster C's, reported not fixed.
* **`draw.c:658/662` remain unpatched**, as the brief states. `draw.c` is held.
* **`draw.c:653` (the live `select_effect` copy) is where the `r_heart_colors`
  flattening is actually reachable.** See §9.5.

## 9.7 Files touched

| file | what |
|---|---|
| `engine_c_wip/src/ability/effects/look.c` | patch 1 (`rb_effect_look_at`); cluster X `card_type` x2 |
| `engine_c_wip/src/ability/resolver.c` | cluster X `per_unit_count` |
| `engine_c_wip/src/ability/effects/score.c` | cluster X `card_type` |
| `engine_c_wip/tests/test_parity_x_class.c` | **new** — 5 cases pinning cluster X |
| `engine_c_wip/tests/test_parity_look_reveal.c` | `test_look_at_counts_and_zones` repaired |
| `engine_c_wip/tests/test_parity_executor_dispatch.c` | 2 look-at assertions + 3 resume calls repaired |
| `engine_c_wip/PARITY_CHECKLIST.md` | cluster **X** |
| `engine_c_wip/NOZOMI_ROOT_CAUSE.md` | this section |

Snapshot-only, never the live tree: `tools/audit_x_{class,reach,perunit,lookat,const}.c`,
all instrumentation of `ability_queue.c` / `triggers.c` /
`game_state_abilities.c`, and patches 2 + 3.

No git writes. `src/abilities_strings.bin` verified 726831 B /
`89204df0bbde08a0` before and after every build; `make data` was never run.
`make data` remains frozen. `sizeof(GameState)` not touched: no `GameState`
field was added.

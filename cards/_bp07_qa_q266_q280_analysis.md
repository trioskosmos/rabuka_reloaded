# BP07 QA entries Q266窶轍280 窶・analysis & test coverage

Source of truth: `cards/qa_data.json` (entries Q266窶轍280, dated 2026.06.26 / 2026.08.05).
Card ability texts: `cards/cards.json` (`ability` field). Parser output / per-ability decoding:
`cards/abilities.json` + `engine/src/ability/abilities_gen.rs`.

This file records, for each QA entry, the underlying game rule, the card/ability involved,
and where (if anywhere) it is already exercised by a gameplay test. Entries marked **gap**
have no test that drives the real engine rule end-to-end.

Legend: `笨・covered` = a gameplay test in `engine/tests/test_modules/` already drives the
specific rule described. `partial` = some adjacent mechanic is tested but the QA edge is not.
`gap` = no test touches it.

---

## Q266 窶・鬯ｼ蝪壼､冗ｾ・PL!SP-pb2-009 ab#0 (逋ｻ蝣ｴ/繝ｩ繧､繝夜幕蟋区凾)
> 逋ｻ蝣ｴ/繝ｩ繧､繝夜幕蟋区凾・壹鮫iella!縲上・繝｡繝ｳ繝舌・1莠ｺ繧偵え繧ｧ繧､繝医↓縺励※繧ゅｈ縺・ｼ夂嶌謇九・繧ｹ繝・・繧ｸ縺ｫ縺・ｋ蜈・・戟縺､繝悶Ξ繝ｼ繝峨・謨ｰ縺後％繧後↓繧医ｊ繧ｦ繧ｧ繧､繝医↓縺励◆繝｡繝ｳ繝舌・縺悟・縲・戟縺､繝悶Ξ繝ｼ繝峨・謨ｰ繧医ｊ2縺､莉･荳雁ｰ代↑縺・Γ繝ｳ繝舌・1莠ｺ繧偵え繧ｧ繧､繝医↓縺吶ｋ縲・- **QA rule**: paying the wait cost with a 0-blade member means the opponent target must have
  `0 - 2` = impossible; so a 0-blade opponent member cannot be waited. Blade comparison is on
  **original blade count** (蜈・・戟縺､), and waited members' blades don't count toward yell reveal.
- **Status**: 笨・covered 窶・`bp7_q266_natsumi_blade_wait_test.rs` (5 tests) drives the real
  blade-0 wait cost boundary (costed B0竊地othing waitable, B2竊鍛lade-0 only, B4竊鍛lade-2/3,
  skip-cost竊地othing).

## Q267 窶・螟ｩ邇句ｯｺ迺・･・PL!N-bp7-009-R ab#0 (逋ｻ蝣ｴ)
> 逋ｻ蝣ｴ・夊・蛻・→逶ｸ謇九・縺昴ｌ縺槭ｌ縲∬・霄ｫ縺ｮ繝・ャ繧ｭ縺ｮ荳翫°繧峨き繝ｼ繝峨ｒ7譫壽而縺亥ｮ､縺ｫ鄂ｮ縺上・- **QA rule**: if the deck runs exactly to 0 during the mill, a refresh happens immediately
  (before the auto-ability/effect continues), so cards leave the waitroom.
- **Status**: 笨・covered 窶・`bp7_q267_rinna_mill_refresh_test.rs` (4 tests) drives the real
  mill through the card's 逋ｻ蝣ｴ (both players each mill 7): deck-0 mid-mill refresh completes
  to 7 total, deck=exactly-7 no-refresh boundary, deck=5 refresh竊痴till 7 total, and both
  players refreshing their own deck.

## Q268 窶・荳芽飴譬槫ｭ・PL!N-bp7-010-R ab#0 (襍ｷ蜍・
> 襍ｷ蜍包ｼ壹お繝阪Ν繧ｮ繝ｼ鄂ｮ縺榊ｴ縺ｫ縺ゅｋ繧ｨ繝阪Ν繧ｮ繝ｼ1譫壹ｒ縺薙・繝｡繝ｳ繝舌・縺ｮ荳九↓鄂ｮ縺擾ｼ夊・蛻・・謗ｧ縺亥ｮ､縺九ｉ繧ｳ繧ｹ繝・莉･荳九・縲手匯繝ｶ蜥ｲ縲上・繝｡繝ｳ繝舌・繧ｫ繝ｼ繝峨ｒ1譫壹√Γ繝ｳ繝舌・縺ｮ縺・↑縺・お繝ｪ繧｢縺ｫ繧ｦ繧ｧ繧､繝育憾諷九〒逋ｻ蝣ｴ縺輔○繧九・- **QA rule**: the cost (place 1 energy under self) can be paid **even when there is no empty
  area** to put the new member 窶・cost is payable independent of whether the effect fully resolves.
- **Status**: 笨・covered 窶・`bp7_q268_shioriko_empty_area_deploy_test.rs` (8 tests) drives the
  real 襍ｷ蜍・ cost is payable with NO empty area (Q268 core), deploy-to-empty-area-in-wait,
  cost>2 / non-陌ｹ繝ｶ蜥ｲ target exclusion, exactly-one-of-two deployed, 繧ｿ繝ｼ繝ｳ1蝗・limit, and
  no-target+no-empty-area cost still paid.

## Q269 / Q277 窶・繝溘い繝ｻ繝・う繝ｩ繝ｼ PL!N-bp7-011-R・・ab#0 (閾ｪ蜍・ + ab#1 (蟶ｸ譎・
> 閾ｪ蜍包ｼ壹％縺ｮ繧ｫ繝ｼ繝峨′繝・ャ繧ｭ縺九ｉ謗ｧ縺亥ｮ､縺ｫ鄂ｮ縺九ｌ縺溘→縺阪∵焔譛ｭ繧・譫壽而縺亥ｮ､縺ｫ鄂ｮ縺・※繧ゅｈ縺・ゅ◎縺・＠縺溘→縺阪∵而縺亥ｮ､縺九ｉ縺薙・繧ｫ繝ｼ繝峨ｒ謇区惆縺ｫ蜉縺医ｋ縲・> 蟶ｸ譎ゑｼ壹％縺ｮ繧ｫ繝ｼ繝峨ｒ繝励Ξ繧､縺吶ｋ髫帙∬・蛻・・謗ｧ縺亥ｮ､縺ｫ縺ゅｋ縺吶∋縺ｦ縺ｮ繝｡繝ｳ繝舌・繧ｫ繝ｼ繝峨ｒ繧ｷ繝｣繝・ヵ繝ｫ縺励√ョ繝・く縺ｮ荳九↓鄂ｮ縺・※繧ゅｈ縺・ゅ◎縺・＠縺溘→縺阪√％縺ｮ繧ｫ繝ｼ繝峨・繧ｳ繧ｹ繝医・・呈ｸ帙ｋ縲・- **Q269 rule**: being revealed by 繧ｨ繝ｼ繝ｫ (yell) is NOT a deck竊蜘aitroom move, so the 閾ｪ蜍・does
  NOT trigger.
- **Q277 rule**: when a mill empties the deck to exactly 0, refresh occurs **before** the 閾ｪ蜍・  ability resolves; the card leaves the waitroom and can no longer be added to hand.
- **Status**: 笨・covered 窶・`bp7_mia_optional_recover_test.rs` covers the optional-discard
  recovery path; `bp7_q269_mia_yell_no_trigger_test.rs` covers the Q269 yell-reveal
  no-trigger edge (3 tests + deck竊壇iscard control) and the Q277 refresh-before-resolve
  edge (2 tests + no-refresh control).
- **ab#1 (蟶ｸ譎・**: 笨・implemented + covered 窶・the play-time "繝励Ξ繧､縺吶ｋ髫帚ｦ繧ｳ繧ｹ繝医・・呈ｸ帙ｋ"
  ability is now wired into `handle_play_member_to_stage` (two-phase hook + resume), offering
  the optional choice on play; accepting shuffles waitroom members to deck bottom and reduces
  the cost by 2. Covered by `bp7_mia_play_cost_reduction_test.rs` (accept/decline/no-waitroom).

## Q270 窶・繧ｨ繝槭・繝ｴ繧ｧ繝ｫ繝・PL!N-bp7-020-N ab#0 (逋ｻ蝣ｴ)
> 逋ｻ蝣ｴ・夊・蛻・・繝・ャ繧ｭ縺ｮ荳翫°繧峨き繝ｼ繝峨ｒ3譫壽而縺亥ｮ､縺ｫ鄂ｮ縺上ゅ◎繧後ｉ縺ｮ繝｡繝ｳ繝舌・繧ｫ繝ｼ繝峨・荳ｭ縺ｫ2遞ｮ鬘樔ｻ･荳翫・繝悶Ξ繝ｼ繝峨ワ繝ｼ繝医・濶ｲ縺後≠繧句ｴ蜷医√Λ繧､繝也ｵゆｺ・凾縺ｾ縺ｧ縲”eart04繧貞ｾ励ｋ縲・- **QA rule**: "2 types of blade-heart color" 窶・an ALL-heart card is not a blade-heart color;
  a green blade-heart is one type. So {ALL-heart, green-blade-heart} = only 1 type 竊・no heart04.
  "2遞ｮ鬘樔ｻ･荳・ = 2 or more distinct **blade-heart** colors.
- **Status**: 笨・covered 窶・`bp7_q270_emma_color_diversity_test.rs` covers the 2-distinct-color rule
  (5 tests: 2/3 distinct 竊・heart04, 1 color 竊・none, single member 竊・none, no blade heart 竊・  none). The ALL-heart-vs-blade-heart counting edge: no member card in the DB has a
  colorless/ALL blade heart, so the "not counted as a blade-heart color" case is subsumed by
  the no-blade-heart test (non-color blade hearts never contribute a distinct color).

## Q271 窶・Colorful Dreams! Colorful Smiles! PL!N-bp7-025-L ab#1 (繝ｩ繧､繝匁・蜉滓凾)
> 繝ｩ繧､繝匁・蜉滓凾・壹お繝ｼ繝ｫ縺ｫ繧医ｊ蜈ｬ髢九＆繧後◆閾ｪ蛻・・繧ｫ繝ｼ繝峨・荳ｭ縺ｫ heart01縲徂eart06 縺ｮ縺・■3遞ｮ鬘樔ｻ･荳翫≠繧句ｴ蜷医√％縺ｮ繧ｫ繝ｼ繝峨・繧ｹ繧ｳ繧｢繧抵ｼ具ｼ代☆繧九・- **QA rule**: cards with only 譯・髱・ALL **blade-heart** do NOT satisfy a "3 types of heart
  color" condition 窶・blade-heart is not a heart color.
- **Status**: 笨・covered 窶・`bp7_q271_colorful_dreams_test.rs` (9 tests) drives ab#1 (Q271: peach/
  blue/ALL blade-hearts 竊・no score; 3 blade-heart colors mapping to heart colors 竊・no score;
  3 distinct base hearts 竊・+1; single multi-heart card 竊・+1; 2 base hearts 竊・no score; blade
  doesn't add a color; empty reveal 竊・no score) and ab#0 (繝ｩ繧､繝夜幕蟋区凾 陌ｹ繝ｶ蜥ｲ member gains 1 blade;
  non-陌ｹ繝ｶ蜥ｲ gains none).
- **Engine fix (Q271)**: `resolve_zone_card_count`'s RevealedCards "types" branch now counts only
  base-heart colors when `heart_source` is not `blade` (previously it also added blade-heart
  colors, so a blade-heart set could wrongly satisfy the condition).

## Q272 窶・Just Believe!!! PL!N-bp7-026-L ab#0 (繝ｩ繧､繝夜幕蟋区凾)
> 繝ｩ繧､繝夜幕蟋区凾・壽焔譛ｭ繧・譫壹∪縺ｧ謗ｧ縺亥ｮ､縺ｫ鄂ｮ縺・※繧ゅｈ縺・ｼ夊・蛻・・繧ｹ繝・・繧ｸ縺ｫ縺・ｋ縲手匯繝ｶ蜥ｲ縲上・繝｡繝ｳ繝舌・繧偵√％繧後↓繧医ｊ謗ｧ縺亥ｮ､縺ｫ鄂ｮ縺・◆繧ｫ繝ｼ繝峨・譫壽焚縺ｫ遲峨＠縺・焚縺ｾ縺ｧ驕ｸ縺ｶ縲ゅΛ繧､繝也ｵゆｺ・凾縺ｾ縺ｧ縲√◎繧後ｉ縺ｯ繝悶Ξ繝ｼ繝峨ｒ蠕励ｋ縲・- **QA rule**: the same member cannot be selected multiple times (no-repeat selection).
- **Status**: 笨・covered 窶・`bp7_q272_just_believe_test.rs` (10 tests) drives ab#0 (Q272: discard-1 竊・1 blade;
  discard-2 竊・2 distinct members each get 1; **1 member + discard 2 竊・exactly 1 blade (cannot select twice)**;
  non-陌ｹ繝ｶ蜥ｲ not selectable; skip discard 竊・no blade) and ab#1 (繝ｩ繧､繝匁・蜉滓凾: 2+ member cards without
  blade-hearts among revealed 竊・+1 score; 1 only / with-blade / mixed / live card 竊・no score).

## Q273 窶・貂｡霎ｺ 譖・PL!S-bp7-005-R・・ab#2 (襍ｷ蜍・繧ｻ繝ｳ繧ｿ繝ｼ)
> 襍ｷ蜍包ｼ壽焔譛ｭ繧・譫壽而縺亥ｮ､縺ｫ鄂ｮ縺擾ｼ壹％縺ｮ繝｡繝ｳ繝舌・縺ｨ閾ｪ蛻・・繧ｹ繝・・繧ｸ縺ｫ縺・ｋ縺ｻ縺九・縲拶qours縲上・繝｡繝ｳ繝舌・1莠ｺ繧帝∈縺ｶ縲ゅ◎繧後ｉ縺梧戟縺､逋ｻ蝣ｴ閭ｽ蜉帙◎繧後◇繧・縺､繧堤匱蜍輔＆縺帙ｋ縲・- **QA rule**: an 逋ｻ蝣ｴ ability activated this way still pays its own cost if it has one.
- **Status**: 笨・covered 窶・`bp7_q273_watanabe_cost_test.rs` (2 tests) drives ab#2 firing both
  selected members' 逋ｻ蝣ｴ abilities and verifies the fired 逋ｻ蝣ｴ ability's cost is offered and
  paid (Q273), plus 貂｡霎ｺ's own 逋ｻ蝣ｴ fires (member placed under her).
- **Engine fix (Q273)**: `execute_activate_ability` previously fired only the LAST selected card's
  ability and executed its effect directly (skipping the cost). It now (a) fires EVERY selected
  member's 逋ｻ蝣ｴ ability (縲後◎繧後ｉ縺娯ｦ縺昴ｌ縺槭ｌ縲・ and (b) enqueues each fired ability through the
  normal queue so its own cost is paid before the effect resolves; the trigger is inferred as 逋ｻ蝣ｴ
  when the parser leaves `target_trigger` null.

## Q274 / Q275 窶・譚ｾ豬ｦ譫懷漉 PL!S-bp7-003-R・・ab#1 option 1 (逋ｻ蝣ｴ)
> 逋ｻ蝣ｴ・壻ｻ･荳九°繧・縺､繧帝∈縺ｶ縲ゅ・繝ｩ繧､繝也ｵゆｺ・凾縺ｾ縺ｧ縲∬・蛻・・繧ｹ繝・・繧ｸ縺ｫ縺・ｋ蜈・・戟縺､繝悶Ξ繝ｼ繝峨・謨ｰ縺・縺､莉･荳九・縲拶qours縲上・繝｡繝ｳ繝舌・縺ｯ縲∫嶌謇九・蜉ｹ譫懊↓繧医▲縺ｦ縺ｯ繧ｦ繧ｧ繧､繝医＠縺ｪ縺・・- **Q274 rule**: an opponent MAY still *select* your wait-immune member as the target of their
  wait effect (selection is allowed; the wait simply doesn't apply). Cannot-immune 竕 cannot-select.
- **Q275 rule**: when an effect forces *you* to wait a member (e.g. 繧ｻ繝ｩ繧ｹ譟ｳ逕ｰ繝ｪ繝ｪ繧ｨ繝ｳ繝輔ぉ繝ｫ繝・  PL!HS-bp6-007-R), a wait-immune member is NOT a legal choice 窶・you must pick a waitable member.
- **Status**: `bp7_kanan_wait_immunity_test.rs` + `bp7_wait_immunity_helpers.rs` (G4) cover the
  immunity blocking an opponent's wait. Q274 (still selectable) now 笨・covered by
  `bp7_q274_immune_still_selectable_test.rs` (3 tests): the wait-immune member is STILL
  offered in the opponent's wait-target choice and, when selected, simply isn't waited
  (selected-immune → stays active; non-immune pick → waited; no-immunity control → waited).

## Q276 窶・Cheer Mode PL!N-bp7-030-L ab#1 (繝ｩ繧､繝匁・蜉滓凾)
> 繝ｩ繧､繝匁・蜉滓凾・壹％縺ｮ繧ｫ繝ｼ繝峨ｒ繝ｩ繧､繝悶き繝ｼ繝臥ｽｮ縺榊ｴ縺九ｉ謇区惆縺ｫ謌ｻ縺吶ゅ◎縺ｮ蠕後∵焔譛ｭ繧・譫壽而縺亥ｮ､縺ｫ鄂ｮ縺上・- **QA rule**: winning a live with only this card still forces it back to hand (ab#1 is
  mandatory), so it cannot be left in the success live-card zone.
- **Status**: `blade_heart_colorless_test.rs` references `PL!N-bp7-030` (colorless/blade-heart
  scope). The success-zone-vs-return-to-hand edge: **engine-confirmed** — LiveSuccess abilities
  resolve (ab#1 returns Cheer Mode to hand) before `move_live_to_success_and_handle_wins`, so the
  card is no longer in the live zone and is never placed in the success zone. Covered by
  `bp7_q276_cheer_mode_return_hand_test`.

## Q278 / Q279 窶・譯懷揩縺励★縺・PL!N-bp7-003-R・・ab#1 (繝ｩ繧､繝夜幕蟋区凾)
> 繝ｩ繧､繝夜幕蟋区凾・壹Λ繧､繝也ｵゆｺ・凾縺ｾ縺ｧ縲√％縺ｮ繝｡繝ｳ繝舌・縺ｮ荳九↓鄂ｮ縺九ｌ縺ｦ縺・ｋ蜷榊燕縺ｮ逡ｰ縺ｪ繧九Γ繝ｳ繝舌・繧ｫ繝ｼ繝・譫壹↓縺､縺阪√ヶ繝ｬ繝ｼ繝峨ｒ蠕励ｋ縲・- **Q278 rule**: under cards = 荳雁次豁ｩ螟｢ `PL!N-bp1-001-R` + 荳雁次豁ｩ螟｢&貔∬ｰｷ縺九・繧・譌･驥惹ｸ玖干蟶・  `LL-bp1-001-R・義 竊・**2 blades**.
- **Q279 rule**: under cards = 荳雁次豁ｩ螟｢ + 貔∬ｰｷ縺九・繧・+ 譌･驥惹ｸ玖干蟶・+ the `LL-bp1-001-R・義 joint
  card 竊・**3 blades**. The joint card does NOT add a 4th distinct name slot when its constituent
  names are already present.
- **Status**: 笨・`bp7_q278_q279_joint_blade_test.rs`.
  - Q278 exact: under = 荳雁次豁ｩ螟｢ `PL!N-bp1-001-R` + joint `LL-bp1-001-R・義 竊・**2 blades**.
  - Q279 exact: under = 荳雁次豁ｩ螟｢ + 貔貔∬ｰｷ縺九・繧・+ 譌･驥惹ｸ玖干蟶・ + the joint 竊・**3 blades**.
  - Engine: new joint-aware counter `count_distinct_member_name_units`
    (`engine/src/ability/util.rs:2270`): ordinary cards dedupe by `normalize_name`; a joint
    (multi-name `A&B&C`) card contributes ONE unit only when it introduces a name NOT already
    present as a single-name card. Wired into the per-unit counts at
    `util.rs:1642` (`count_matching_distinct`) and `effects/state.rs:52`.
  - Tests: exact Q278, exact Q279, singles-only control (3), partial coverage (歩下+かのん+joint
    =3), joint alone (=1), duplicate singles dedupe (=2), empty under (=0).

## Q280 窶・邀ｳ螂ｳ繝｡繧､ PL!SP-bp7-007-R・・ab#1 (繝ｩ繧､繝匁・蜉滓凾)
> 繝ｩ繧､繝匁・蜉滓凾・夊・蛻・・繧ｨ繝阪Ν繧ｮ繝ｼ繝・ャ繧ｭ縺九ｉ縲√お繝阪Ν繧ｮ繝ｼ繧ｫ繝ｼ繝峨ｒ2譫壹え繧ｧ繧､繝育憾諷九〒鄂ｮ縺上ゅ◎繧後ｉ縺ｮ繧ｨ繝阪Ν繧ｮ繝ｼ繧ｫ繝ｼ繝峨・縲∵ｬ｡縺ｮ繧ｿ繝ｼ繝ｳ縺ｮ繧｢繧ｯ繝・ぅ繝悶ヵ繧ｧ繧､繧ｺ縺ｫ繧｢繧ｯ繝・ぅ繝悶＠縺ｪ縺・・- **QA rule**: an energy whose "do-not-activate" (繧｢繧ｯ繝・ぅ繝悶＠縺ｪ縺・ is in force stays non-activating
  next turn even if it was moved / a live-success ability tried to activate it; the
  do-not-activate effect persists until its end condition.
- **Status**: **gap**. No test references `PL!SP-bp7-007`.

---

## Also: PL!N-sd2 rows already marked in `_bp07_ability_gaps_hand_analysis.md`

The verdict table rows 54窶・3 (`PL!N-sd2-001/006/010/013/015/017/019/021` + `PL!N-sd2-026`) are
marked 笨・there, but that marks **parser faithfulness only** (the parser produced correct JSON),
NOT gameplay-test coverage. Coverage status of those sd2 cards in
`engine/tests/test_modules/`:

| Card | Ability | md verdict | Test file reference |
|------|---------|-----------|---------------------|
| PL!N-sd2-026-P Fire Bird ab#0 | blade竕･4 竊・heart02ﾃ・ | 笨・(CLEAN-G14) | `bp7_fire_bird_blade_gain_test.rs` |
| PL!N-sd2-001-SD2 荳雁次豁ｩ螟｢ ab#0 | E2 竊・陌ｹ繝ｶ蜥ｲ live to hand | 笨・(E2) | referenced in `bp7_audrey_blade_max_test.rs` |
| PL!N-sd2-006-SD2 霑第ｱ溷ｽｼ譁ｹ ab#0 | wait 陌ｹ繝ｶ蜥ｲ 竊・blade2 | 笨・| **no test** (gap) |
| PL!N-sd2-010-SD2 荳芽飴譬槫ｭ・ab#0/1 | draw2 / wait竊壇iscard竊誕ctive+blade2 | 笨・| **no test** (gap) |
| PL!N-sd2-013-SD2 荳雁次豁ｩ螟｢ ab#0 | 陌ｹ繝ｶ蜥ｲ only 竊・opp blade竕､2 wait | 笨・| **no test** (gap) |
| PL!N-sd2-015-SD2 譯懷揩縺励★縺・ab#0 | wait + discard 竊・draw | 笨・| **no test** (gap) |
| PL!N-sd2-017-SD2 螳ｮ荳区・ ab#0 | E optional 竊・active 1 | 笨・| **no test** (gap) |
| PL!N-sd2-019-SD2 蜆ｪ譛ｨ縺帙▽闖・ab#0/1 | heart05 / opp cost竕､2 wait | 笨・| **no test** (gap) |
| PL!N-sd2-021-SD2 螟ｩ邇句ｯｺ迺・･・ab#0 | opp cost竕､4 wait | 笨・| **no test** (gap) |

So the 笨・verdicts in `_bp07_ability_gaps_hand_analysis.md` are NOT test coverage 窶・those sd2
cards (except Fire Bird) have no gameplay test.

---

## Summary of gaps to close (Q266窶轍280)

| QA | Card | Rule | Status |
|----|------|------|--------|
| Q266 | PL!SP-pb2-009 鬯ｼ蝪壼､冗ｾ・| blade-0 wait cost 竍・cannot wait 0-blade opp | 笨・`bp7_q266_natsumi_blade_wait_test` |
| Q267 | PL!N-bp7-009 螟ｩ邇句ｯｺ迺・･・| deck-to-0 refresh mid-mill | 笨・`bp7_q267_rinna_mill_refresh_test` |
| Q268 | PL!N-bp7-010 荳芽飴譬槫ｭ・| cost payable with no empty area | 笨・`bp7_q268_shioriko_empty_area_deploy_test` |
| Q269 | PL!N-bp7-011 繝溘い | yell reveal does not trigger 閾ｪ蜍・| 笨・`bp7_q269_mia_yell_no_trigger_test` |
| Q270 | PL!N-bp7-020 繧ｨ繝・| ALL-heart not a blade-heart color | 笨・`bp7_q270_emma_color_diversity_test` |
| Q277 | PL!N-bp7-011 繝溘い | refresh before 閾ｪ蜍・resolve | 笨・`bp7_q269_mia_yell_no_trigger_test` |
| Q271 | PL!N-bp7-025 Colorful Dreams | blade-heart 竕 heart color (score) | 笨・`bp7_q271_colorful_dreams_test` |
| Q272 | PL!N-bp7-026 Just Believe | no-repeat select | 笨・`bp7_q272_just_believe_test` |
| Q273 | PL!S-bp7-005 貂｡霎ｺ譖・| activated 逋ｻ蝣ｴ ability pays cost | 笨・`bp7_q273_watanabe_cost_test` |
| Q274 | PL!S-bp7-003 譚ｾ豬ｦ譫懷漉 | wait-immune member still selectable | 笨・`bp7_q274_immune_still_selectable_test` |
| Q275 | PL!S-bp7-003 譚ｾ豬｡譫懷漉 | not a legal forced-wait choice | 手・`bp7_q275_forcepick_wait_test` |
| Q276 | PL!N-bp7-030 Cheer Mode | return-to-hand beats success zone | 手・`bp7_q276_cheer_mode_return_hand_test` |
| Q278 | PL!N-bp7-003 譯懷揩縺励★縺・| joint card = 2 blades | 手・`bp7_q278_q279_joint_blade_test` |
| Q279 | PL!N-bp7-003 譯懷揩縺励★縺・| joint card 竕 extra distinct name | 手・`bp7_q278_q279_joint_blade_test` |
| Q280 | PL!SP-bp7-007 邀ｳ螂ｳ繝｡繧､ | do-not-activate persists | **gap** |

Plus the 8 PL!N-sd2 cards (rows 55窶・3) marked 笨・parser-only that have **no gameplay test**.

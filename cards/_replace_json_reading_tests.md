# JSON-Reading Tests → Replace with 5 Real Gameplay Edge-Case Tests

The following test files I added this session only assert the **parsed JSON shape**
(reading `effect.action`, `optional_action`, `condition.unit`, etc.) rather than
exercising actual gameplay. Each must be rewritten as **5 real gameplay edge-case
tests** (set up a board, trigger the ability, resolve choices, assert game-state
outcome). This doc lists each offending test file, its ability, and the 5 edge
cases the replacement must cover.

## 1. G13 — Like a Treasure ab#1 (自動) — `bp7_like_a_treasure_optional_test.rs`

Ability:
> 自分のライブ成功時能力によって、カードが自分のデッキから自分の控え室に置かれるたび、
> それらのカードの中から『虹ヶ咲』のライブカードを1枚手札に加えてもよい。そうしたとき、
> このカードのスコアを+1する。

Offending tests (JSON-reading): `like_a_treasure_ab1_is_conditional_optional`,
`like_a_treasure_optional_action_moves_niji_live_to_hand`,
`like_a_treasure_accepted_branch_moves_then_scores`.

5 gameplay edge cases:
1. A 『虹ヶ咲』 live card is among the moved deck→discard cards and the player accepts (Pay) → the live card is added to hand **and** the live card's score +1.
2. The player declines (Skip) → nothing is added to hand and no score is gained.
3. No 『虹ヶ咲』 live card among the moved cards → even on accept, nothing is added (the group/card-type filter excludes it).
4. Multiple 『虹ヶ咲』 live cards are among the moved cards → exactly **one** is added (count 1), score +1.
5. The trigger only fires for the qualifying deck→discard movement: a movement that isn't a card placed into discard does NOT offer the add-to-hand optional.

## 2. G16 — 未来の音が聴こえる ab#0 (ライブ開始時) — `bp7_mirai_no_oto_optional_test.rs`

Ability:
> 自分の控え室にある『Liella!』のメンバーカードを9枚選び、それらをシャッフルし、
> デッキの一番下に置いてもよい。そうしたとき、ライブ終了時まで、自分のステージにいる
> すべてのメンバーはブレードを得る。

Offending tests (JSON-reading): `mirai_ab0_is_conditional_optional`,
`mirai_optional_action_shuffles_discard_to_deck_bottom`,
`mirai_accepted_branch_moves_then_grants_blade`.

5 gameplay edge cases:
1. Exactly 9 『Liella!』 member cards in discard, accept (Pay) → the 9 leave the discard and appear on the **bottom** of the deck, and every stage member gains +1 blade.
2. Decline (Skip) → discard is untouched and no member gains blade.
3. Fewer than 9 『Liella!』 members in discard → only the available ones can be moved (and the blade consequence follows doing it).
4. Non-『Liella!』 cards in discard are NOT selectable/moved; the shuffle only touches the chosen 『Liella!』 members.
5. The blade gain is "until live end" and applies to **all** stage members (not just 『Liella!』 ones).

## 3. G18 — エマ・ヴェルデ ab#0 (登場) — `bp7_emma_color_diversity_test.rs`

Ability:
> 自分のデッキの上からカードを3枚控え室に置く。それらのメンバーカードの中に2種類以上の
> ブレードハートの色がある場合、ライブ終了時まで、heart04を得る。

Offending tests (JSON-reading): `emma_color_diversity_uses_types_unit_count_2`,
`emma_mill_step_top3_to_discard`.

5 gameplay edge cases:
1. The top 3 milled cards are members with **2 distinct** blade-heart colors → heart04 is gained.
2. The top 3 milled cards are members with **3 distinct** blade-heart colors → heart04 is gained (≥2).
3. All milled members share **1** blade-heart color → no heart04.
4. Only **1** of the 3 milled cards is a member (others are non-member); even if it has a blade-heart color → no heart04 (needs ≥2 distinct among member cards).
5. Milled members with **no** blade heart (only base heart) don't count toward color diversity → no heart04.

## 4. G19 — 渡辺 曜 ab#2 (起動) — `bp7_watanabe_select_self_and_other_test.rs`

Ability:
> 手札を2枚控え室に置く：このメンバーと自分のステージにいるほかの『Aqours』のメンバー1人を
> 選ぶ。それらが持つ登場能力それぞれ1つを発動させる。

Offending test (borderline — checks the select count after activating):
`watanabe_select_includes_this_member`.

5 gameplay edge cases:
1. This member + 1 other Aqours member are both on stage and both have 登場 abilities → after the cost, BOTH 登場 abilities fire.
2. Only this member (no other Aqours member) is on stage → this member's own 登場 ability still fires.
3. This member has no 登場 ability, the other Aqours member does → only the other's 登場 ability fires.
4. The selection must include this member: the select targets 2 cards (this member is a mandatory/selectable candidate, not excluded).
5. Each selected member's 登場 ability fires exactly once (no double-fire).

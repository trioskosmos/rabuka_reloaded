# Test Plan: La Bella Patria (PL!N-bp3-027-L)

## Ability Text (full)

```
{{live_success.png|ライブ成功時}}
このターン、自分が余剰ハートに
{{heart_04.png|heart04}}
を1つ以上持っており、かつ自分のステージに『虹ヶ咲』のメンバーがいる場合、
自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
```

On live success: if this turn you have 1+ surplus heart04 AND you have a 虹ヶ咲 member on your stage, place 1 energy card from your energy deck in wait state.

## Condition breakdown (compound: AND)

1. **Surplus heart04 ≥ 1**  
   `resource_type: surplus_heart, heart_colors: ["heart04"], count: 1, operator: >=`

2. **虹ヶ咲 member on stage**  
   `type: group_condition, group_names: ["虹ヶ咲"]`

---

## Q174 (from qa_data.json)

```
Question:
「
(ability text)
」について、ステージに緑ハートのないエールにしかALLハートを3つ持たせて
ライブを成功させた場合、ライブ成功時の能力は使えますか？

Answer:
いいえ。使えません。
```

**Translation**:  
"If yell cards have NO green heart04, only 3 ALL hearts, and you succeed the live — can the live success ability be used?"

**Answer**: No, it cannot be used.

### Test: No heart04 → ability does NOT fire

| Element | Setup |
|---------|-------|
| Stage card | A member providing hearts but **NO heart04** (e.g. heart01+heart03+heart06) |
| Group condition | Stage has **NO 虹ヶ咲 member** OR the member literally provides 0 heart04 |
| Live card | La Bella Patria (needs heart03=2, heart04=2, heart0=1) |
| Expected result | Energy zone stays 0 — ability does NOT fire |

**Why**: The compound condition requires surplus heart04 ≥ 1. If the member provides 0 heart04 (and no other source adds heart04), heart04 surplus = 0 → condition fails. The live itself must still succeed (enough hearts via wildcards) for LiveSuccess to trigger.

**Current test (`bella_q174_no_heart04_surplus`)** passes for the wrong reason. It uses `PL!N-sd1-015-SD` whose series does NOT map to `"虹ヶ咲"` (its series is `"スクールアイドルフェスティバル"`, which maps to empty group). So the **group condition** fails, not the surplus condition.

**Fix**: Use a member that:
- Has series mapping to `"虹ヶ咲"` (series = `"ラブライブ！虹ヶ咲学園スクールアイドル同好会"`)  
- Provides some hearts but **NOT heart04**
- OR: Accept the group condition failure as a separate test and create a proper heart04-specific test separately

---

## Q173 (from qa_data.json)

```
Question:
「
(ability text)
」について、この能力を持つカードを2枚同時にライブ成功させました。
この時、余剰ハートに
{{heart_04.png|heart04}}
が1つの場合、それぞれの能力は使用できますか？

Answer:
はい。可能です。
```

**Translation**:  
"If you simultaneously succeed 2 live cards with this ability, and there is exactly 1 surplus heart04 — can each ability be used?"

**Answer**: Yes, it's possible.

### Test: 2 live cards succeed with surplus heart04 = 1 → both fire

| Element | Setup |
|---------|-------|
| Stage | 虹ヶ咲 member providing enough hearts for both lives |
| Live zone | 2 copies of La Bella Patria |
| Surplus heart04 | Exactly 1 (member provides heart04=5, both lives need heart04=2 each = 4 total → surplus = 1) |
| Expected | Both abilities fire → 2 energy cards placed |

**Why**: Each live card independently checks the surplus condition on its own LiveSuccess trigger. The condition checks "this turn, if you have 1+ surplus heart04" — which is true at the time each trigger fires.

---

## Cards needed

To properly test these, I need members that:

1. **Have group = 虹ヶ咲** (series = `ラブライブ！虹ヶ咲学園スクールアイドル同好会`)
2. **Provide specific hearts** to control surplus

Available 虹ヶ咲 members and their base hearts:
| Card | Name | base_heart | total | h04 |
|------|------|-----------|-------|-----|

(To be filled with actual card data)

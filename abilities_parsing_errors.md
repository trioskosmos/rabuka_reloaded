# Abilities.json Parsing Errors

## Summary
Found 25+ parsing inconsistencies across the abilities.json file (19118 lines).

## Error Categories

### 1. Truncated/Incomplete Effect Text
These entries have effect text that is cut off mid-sentence or incomplete.

- **Line 11911**: Condition text truncated at "回答がそれ以外の"
  - Card: PL!N-PR-022-PR | エマ・ヴェルデ (ab#0)
  - Issue: Condition text is incomplete

- **Line 12432**: Effect text truncated at "(エールをすべて行った後"
  - Card: PL!SP-bp1-027-L | Sing！Shine！Smile！ (ab#0)
  - Issue: Effect text cuts off at parenthetical start

- **Line 12438**: Effect text is "カードを1枚引く。）"
  - Card: PL!SP-bp1-027-L | Sing！Shine！Smile！ (ab#0)
  - Issue: Effect text ends with closing parenthesis without opening

- **Line 12494**: Effect text truncated at "(エールをすべて行った後"
  - Card: PL!HS-bp1-022-L | AWOKE (ab#0)
  - Issue: Effect text cuts off at parenthetical start

- **Line 12500**: Effect text is "カードを1枚引く。）"
  - Card: PL!HS-bp1-022-L | AWOKE (ab#0)
  - Issue: Effect text ends with closing parenthesis without opening

- **Line 18304**: full_text is "とき、エネルギーを2枚アクティブにする。"
  - Card: PL!HS-sd1-001-SD | 日野下花帆 (ab#1)
  - Issue: Appears to be continuation from previous entry (line 18280), missing beginning

### 2. Effect Text Starting with Commas
These entries have effect text that incorrectly starts with a comma, suggesting the parser included the comma from a sequential action but not the preceding text.

- **Line 11292**: "、{{heart_05.png|heart05}}を得る"
  - Card: PL!-bp5-333-R/P+ | 統堂英玲奈 (ab#1)
  - Issue: Effect text starts with comma

- **Line 11574**: "、これにより公開したカードを自分の成功ライブカード置き場に置く"
  - Card: PL!-bp5-334-R/P+ | 櫻内梨子 (ab#0)
  - Issue: Effect text starts with comma (sequential action)

- **Line 11884**: "、自分はカードを1枚引く"
  - Card: PL!N-pb1-009-R/P+ | 中須かすみ (ab#0)
  - Issue: Effect text starts with comma (sequential action)

- **Line 15297**: "、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る"
  - Card: PL!-bp4-018-N | 矢澤にこ (ab#0)
  - Issue: Effect text starts with comma

- **Line 15326**: "、自分の成功ライブカード置き場にあるこのカードのスコアを＋5する"
  - Card: PL!-bp4-019-L | Angelic Angel (ab#0)
  - Issue: Effect text starts with comma

- **Line 15400**: "、自分のセンターエリアにいる『μ's』のメンバーは{{icon_blade.png|ブレード}}を得る"
  - Card: PL!-bp4-020-L | Love wing bell (ab#1)
  - Issue: Effect text starts with comma

- **Line 16099**: "、{{heart_06.png|heart06}}を得る"
  - Card: PL!SP-bp4-021-N | ウィーン・マルガレーテ (ab#0)
  - Issue: Effect text starts with comma

- **Line 17374**: "、{{heart_03.png|heart03}}を得る"
  - Card: PL!SP-bp5-012-N | 澁谷かのん (ab#0)
  - Issue: Effect text starts with comma

- **Line 17514**: "、手札にあるこのメンバーカードのコストは2減る"
  - Card: PL!SP-bp5-017-N | 桜小路きな子 (ab#0)
  - Issue: Effect text starts with comma

- **Line 18269**: "、相手はカードを1枚引く"
  - Card: PL!HS-bp1-020-L | 花帆の歌 (ab#1)
  - Issue: Effect text starts with comma (sequential action)

### 3. Incorrect/Wrong Effect Text
These entries have effect text that doesn't match the full_text description.

- **Line 10621**: Effect text is "下に置かれているエネルギーカードはエネルギーデッキに戻す。）"
  - Card: PL!N-pb1-011-R/P+ | ミア・テイラー (ab#1)
  - Issue: This is just the parenthetical note, not the actual effect. Should be "自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える"

- **Line 11906**: Effect text is "何もしない。\n（相手を傷つけないよう、やさしく、愛をこめて魔法のパンチをすること。）"
  - Card: PL!N-PR-022-PR | エマ・ヴェルデ (ab#0)
  - Issue: Effect text shows wrong branch of conditional effect (the "do nothing" case instead of the actual effect)

- **Line 15846**: Effect text is "以"
  - Card: PL!N-bp4-030-L | Daydream Mermaid (ab#0)
  - Issue: primary_effect text is just one character "以", incomplete

### 4. Parenthetical Notes Not Separated Properly
These entries have parenthetical notes embedded in effect text that should be in a separate `parenthetical` field.

- **Line 12362**: Effect text contains parenthetical that should be in separate field
  - Card: PL!SP-bp1-024-L | Tiny Stars (ab#1)
  - Issue: Effect text includes "(必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。)" which should be in parenthetical field

- **Line 12390**: Effect text contains parenthetical that should be in separate field
  - Card: PL!SP-bp1-026-L | 未来予報ハレルヤ！ (ab#0)
  - Issue: Effect text includes "(必要ハートを確認する時、エールで出た{{icon_b_all.png|ALLブレード}}は任意の色のハートとして扱う。)" which should be in parenthetical field

### 5. Other Issues
- **Line 15789**: primary_effect text is "自"
  - Card: PL!N-bp4-028-L | CHASE!!! (ab#0)
  - Issue: Incomplete text (just one character)

- **Line 16510**: primary_effect text is "このターン、"
  - Card: PL!N-pb1-037-L | Cara Tesoro (ab#0)
  - Issue: Incomplete text

## Investigation Notes

### Root Cause Analysis
The errors suggest the parser has issues with:
1. **Parenthetical handling**: Not properly separating parenthetical notes from main effect text
2. **Sequential actions**: When breaking down sequential actions, commas are being left at the start of subsequent actions
3. **Text truncation**: Some entries are being cut off mid-sentence, possibly due to buffer limits or parsing errors
4. **Split abilities**: Some abilities appear to be split across multiple entries (e.g., line 18304)

### Fix Strategy
1. For comma-starting effects: Remove leading comma and merge with previous action if applicable
2. For truncated text: Reconstruct from full_text field
3. For incorrect effects: Replace with correct text from full_text
4. For parentheticals: Extract to separate parenthetical field
5. For split abilities: Merge related entries

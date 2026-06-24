# Missing Mechanics Found by Text→JSON Comparison

## 1. Conditional Fallback (conditional_alternative) — 10 abilities
Text describes "if X, do Y; if not, do Z" but parsed JSON lacks `conditional_alternative`.

| Card | Text |
|------|------|
| PL!-bp4-005-R+ 星空 凛 | "if no member with 5+ blades → position change" |
| PL!N-PR-003-PR 上原歩夢 | "if no live card in revealed hand → look at 5 more from deck" |
| PL!N-bp5-010-R 三船栞子 | "if no excess heart → +1 score; if 2+ excess → -1 score" — two outcomes |
| PL!SP-bp1-001-R 澁谷かのん | "if no other member → can't live" (restriction misclassified) |
| PL!SP-bp2-023-L リスタート | "if own live cards < opponent → +1 score" |
| PL!S-bp3-005-R 渡辺 曜 | "if own revealed < opponent's → draw 1" |
| PL!N-bp4-001-R 上原歩夢 | "if own energy < opponent's → place energy" |
| PL!N-PR-022-PR エマ | choice-based "ask opponent, branch on answer" |
| PL!-bp3-025-L タカラモノズ | "if no excess heart this turn → +1 score" |
| PL!N-bp5-030-L 繚乱！ | "if member has no all-heart → give all-heart" |

## 2. Placement Order Missing — 1 ability
Text says "in any order" but no `placement_order` field.
- **PL!N-bp3-009-R+** 天王寺璃奈: "好きな順番でデッキの一番下に"

## 3. Next-Turn Temporal Missing — 1 ability
- **PL!HS-bp6-006-R+** 安養寺 姫芽: "次のターンのアクティブフェイズにアクティブしない" — unique mechanic

## 4. Exclude Self Missing — 1 ability
- **PL!N-bp3-008-R+** エマ: "このメンバー以外" — no `exclude_self: true`

## 5. Choice vs Select — 2 abilities
"Choose one of the following" parsed as `select` instead of `choice`.
- **PL!-pb1-001-R** 高坂穂乃果: "choose one: live card OR cost 10+ member"
- **PL!SP-pb2-002-R** 唐 可可: "choose from options, with conditional replacement"

## 6. Area Icons Completely Dropped — 10 abilities
`{{center.png}}`, `{{leftside.png}}`, `{{rightside.png}}` restrictions not captured in JSON.
- **PL!SP-bp4-008-R+** 若菜四季 (ab#0): leftside
- **PL!SP-bp4-008-R+** 若菜四季 (ab#1): rightside
- **PL!SP-bp5-011-R** 鬼塚冬毬 (ab#0): leftside
- **PL!SP-bp5-011-R** 鬼塚冬毬 (ab#2): rightside
- **PL!SP-bp4-003-R** 嵐 千砂都 (ab#0): leftside + rightside
- **PL!SP-pb2-035-N** 唐 可可: leftside
- **PL!SP-pb2-036-N** 嵐 千砂都: rightside
- **PL!SP-pb2-037-N** 平安名すみれ: leftside
- **PL!SP-pb2-041-N** 若菜四季: rightside
- **PL!SP-bp5-011-R** 鬼塚冬毬: also both sides

## 7. Cost Choice — 1 ability
"Choose between two costs to activate" — parser doesn't model this.
- **PL!SP-bp5-001-R+** 澁谷かのん (ab#3): "このメンバーをウェイトにするか、手札を1枚控え室に置く"

## 8. Replacement Effects — 8 abilities
"代わりに" (instead/replacement) not parsed as replacement effects.
- **PL!S-bp2-008-R+** 小原鞠莉 (ab#1): "if 3+ live cards → +2 instead of +1"
- **PL!-pb1-004-R** 園田海未: "if 2+ → +2 instead of +1"
- **PL!SP-pb2-002-R** 唐 可可: "if discarded card has no blade-heart → choose more instead"
- **PL!N-bp3-026-L**: "if both exist → +2 instead of +1"
- **PL!N-bp4-028-L**: "if 6+ → +2 instead of +1"
- **PL!N-bp4-030-L**: "if successful live → choose more instead"
- **PL!N-pb1-037-L**: "if also activated members → +2 instead of +1"
- **PL!-bp6-024-L**: "may replace placement with card from discard"

## 9. Ignored Gameplay Restrictions — 1+ abilities
Constraints in parenthetical text that affect gameplay.
- **PL!S-sd1-006-SD** 津島善子: "this turn, members can't appear in areas where this effect placed a member"

## 10. Scoring Replacement — discovered
- **PL!N-bp3-026-L** etc: "if both exist → instead score +2" — conditional score result

## 11. Non-stacking — none found (parser handles correctly when present)

## Patterns for Further Investigation
- "この効果で" (with this effect) — effect chaining
- 'energyをX枚アクティブ' (activate energy) — should be change_state with state_change
- Quoted ability text with {{icons}} inside "「...」を得る" — the inner text may carry important constraints

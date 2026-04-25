# Parser and Engine Issues - Comprehensive Analysis

## Overview
This document contains all identified parser and engine issues from the systematic re-analysis of abilities.json starting from line 8911.

---

## PARSER ISSUES

### Choice Mechanisms Missing
- Line 544-566: Missing choice mechanism for heart type selection
- Line 753: Missing choice mechanism for heart color selection
- Line 1112: Missing choice mechanism for heart color selection
- Line 1869: Missing choice mechanism for heart type selection
- Line 8343: Missing heart_type choice mechanism
- Line 9068: Missing 'select and reveal from hand' mechanism
- Line 9538-9566: Missing 'self or opponent' choice mechanism (select action incomplete)

### Compound Conditions Missing
- Line 1184: Missing compound condition for cheer+hand size
- Line 2759: Missing compound condition (baton touch + energy)
- Line 3159: Missing compound condition (center + cost equality)
- Line 3649: Missing compound condition (moves OR energy placed)
- Line 7015: Missing compound condition (self 0 AND opponent 1+)
- Line 9385-9426: Compound condition correctly parsed (verified)

### Conditional Logic Missing
- Line 1650: Missing cost comparison in baton touch condition
- Line 2278: Missing multi-branch conditional based on cost totals
- Line 3006: Missing conditional branching based on cost result
- Line 3057: Missing dynamic cost calculation based on groups
- Line 3563: Missing conditional effects based on cost result
- Line 3757: Missing cost result reference for live card check
- Line 4221: Missing conditional effect based on cost result (live card)
- Line 5162: Missing +1 case (no surplus hearts), multi-branch conditional
- Line 8238: Missing conditional effect based on selected card properties

### Exclusion Filters Missing
- Line 6104: Missing 'other' (ほかの) exclusion filter
- Line 6928: Missing 'other' (ほかの) exclusion filter
- Line 6981: Missing 'excluding Onitsuka Fumari' filter
- Line 8530: Missing 'excluding ミア・テイラー' filter
- Line 5513: Missing non-Series Bouquet exclusion filter
- Line 3195: Missing appeared-this-turn filter, non-Aqours targeting

### Dynamic Counts/Calculations Missing
- Line 1991: per_unit should reference cost result
- Line 2059: Missing cost calculation (cost+2)
- Line 2426: Missing dynamic count based on opponent wait members
- Line 2945: Missing dynamic count (live score + 2)
- Line 3444: Missing dynamic count (energy under member + 1)
- Line 3695: per_unit should reference cost result
- Line 4647: Count should reference cost result (discarded cards)
- Line 4964: Missing dynamic cost (energy = selected card score)
- Line 5406: Missing repeat mechanic (up to 4 more times), conditional effect
- Line 6513: Draw count should reference discarded count
- Line 8887: Missing dynamic cost calculation (3 - success_live_card_zone count)
- Line 8913: per_unit should reference cost result (members put to wait)

### Distinct Name/Group Checks Missing
- Line 2656: Missing distinct card name condition
- Line 2974: Missing distinct names check
- Line 5196: Missing distinct card names, distinct group names checks
- Line 3792: Missing contains all card names matching logic
- Line 9007: Missing 'distinct names' filter

### Multi-Trigger Separation Missing
- Line 5842: Missing multi-trigger separation (登場 + ライブ開始時)
- Line 7673: Missing multi-trigger separation (ライブ開始時 + ライブ成功時)
- Line 9109-9143: Multi-trigger (登場 + ライブ開始時) correctly parsed (verified)

### Sequential Structure Issues
- Line 1719: Missing discard after draw
- Line 2399: Incorrectly combined draw and change_state
- Line 3994: Choice option incorrectly parsed as discard instead of sequential
- Line 8068: Missing sequential with 3 separate moves
- Line 8144: Missing reveal action, sequential structure incomplete
- Line 8286: Missing reveal action, sequential structure incomplete
- Line 8586: Missing reveal action, sequential structure incomplete
- Line 9198: Missing reveal action, sequential structure incomplete
- Line 9362: Look_action incorrectly has card_type filter
- Line 9369: Missing reveal action, sequential structure incomplete

### Reveal Until Condition Loop Missing
- Line 6241: Missing 'reveal until live card' loop mechanic
- Line 8655: Missing 'reveal until condition' loop mechanic

### Heart/Blade Context Missing
- Line 3079: Missing cheer-revealed cards context
- Line 3236: Missing heart variety check, cheer-revealed context
- Line 3281: Missing no-blade-hearts check, baton touch tracking
- Line 7084: Missing cheer-revealed cards context
- Line 7110: Missing cheer-revealed context, full re-yell mechanic
- Line 7756: Missing cheer-revealed cards context
- Line 7673: Missing cheer-revealed context
- Line 8168: Missing cheer-revealed cards count context

### Baton Touch Context Missing
- Line 2711: Missing baton touch from 2 members condition
- Line 7553: Missing 'lower cost' and group filter in baton touch condition
- Line 7585: Missing 'lower cost' and group filter in baton touch condition
- Line 9249: Appearance_condition missing 'from Printemps' and 'baton touch' context

### Cost/Score Filters Missing
- Line 3590: Missing cost 10 filter
- Line 4793: Missing 2+ heart04 filter
- Line 5998: Missing 'all have heart04' condition
- Line 6052: Missing 'all have heart01' condition
- Line 6140: Missing 'total cost ≤4' constraint
- Line 6165: Missing 'lower cost than discarded card' comparison
- Line 6426: Missing 2E payment in cost
- Line 6469: Missing 2E payment in cost
- Line 6642: Missing 2E payment in cost
- Line 6679: Missing 'unless 2E paid' conditional cost
- Line 7224: Missing 'center has highest cost' comparison
- Line 8933: Missing 'original blade count ≤1' filter
- Line 9007: Missing 'cost ≤4' filter
- Line 9120: Missing 'BiBi group' filter in cost
- Line 9362: Look_action incorrectly has card_type filter

### Group Filters Missing
- Line 5920: Missing same group name filter
- Line 5574: Missing same group name filter
- Line 5310: Missing 1 per group name distinct selection logic
- Line 3908: Missing same group as discarded card targeting
- Line 8755: 'Printemps group' filter not captured
- Line 8887: Missing 'other lilywhite member' filter

### Position/Area Context Missing
- Line 2875: Custom condition placeholder for position change
- Line 2896: Missing base blades distinction
- Line 3934: Missing area with group condition in position_change
- Line 6904: Missing position change swap mechanic details
- Line 7337: Missing position change swap mechanic details
- Line 8635: Missing 'center' position requirement in cost
- Line 8696: Missing 'original blade count ≤3' filter
- Line 8726: 'opponent's stage' and 'wait state' filters not captured
- Line 8857: 'opponent's stage' and 'wait state' filters not captured
- Line 9277: empty_area destination not implemented in engine

### Action Destination Issues
- Line 156: look_and_select destination should be hand not discard
- Line 1159: Destination should be hand, missing heart filter
- Line 4288: select_action destination should be hand not discard
- Line 4454: select_action destination should be hand not discard
- Line 4754: select_action destination should be hand not discard
- Line 5132: select_action destination should be hand not discard
- Line 5365: select_action destination should be hand not discard
- Line 5700: select_action destination should be hand not discard
- Line 5746: select_action destination should be hand not discard
- Line 6336: select_action destination should be hand not discard
- Line 6444: select_action destination should be hand not discard
- Line 6578: select_action destination should be hand not discard
- Line 6622: select_action destination should be hand not discard
- Line 7174: select_action destination should be hand not discard
- Line 7260: select_action destination should be hand not discard
- Line 7304: select_action destination should be hand not discard

### Other Parser Issues
- Line 1334: Missing invalidate_ability action
- Line 1692: Missing same-name matching logic
- Line 1893: Select action missing options array
- Line 2378: Missing condition for no μ's member with 5+ blades
- Line 2481: Missing both players energy total in condition
- Line 2583: Missing select live card mechanism
- Line 3107: Missing no-ability member check
- Line 3334: Missing same count comparison between players
- Line 3368: surplus_heart condition marked as custom
- Line 3407: place_energy_under_member cost marked as custom
- Line 3818: Missing different costs check
- Line 4121: Missing compound condition, reveal all incomplete, exclude_self in deck look
- Line 4163: Missing cheer-revealed source, OR condition (cost/score)
- Line 4494: Missing active state destination
- Line 4616: Missing required hearts filter (3+ heart06)
- Line 4682: exclude_self not properly integrated with targeting
- Line 5003: Missing base blades distinction, parenthetical note
- Line 5075: place_energy_under_member cost marked as custom
- Line 5271: Position change condition marked as custom
- Line 5433: Missing center targeting, both players targeting
- Line 5543: Missing select member, dynamic cost setting, conditional effect
- Line 6363: Missing first unconditional draw
- Line 6534: Missing 'all are member cards' condition
- Line 6754: Missing 'only' (のみ) condition, rotation pattern, both players targeting
- Line 7403: comparison_target field not implemented in engine
- Line 7513: destination_choice field not implemented in engine
- Line 7616: comparison_target field not implemented in engine
- Line 7643: 'opponent didn't discard' condition marked as custom
- Line 7699: comparison_target field not implemented in engine
- Line 7733: 'opponent didn't discard' condition marked as custom
- Line 7785: Missing 'self or opponent' choice mechanism
- Line 7825: Missing 'both players' targeting
- Line 7843: 'opponent's stage' and 'wait state' filters not captured
- Line 8029: 'success live card zone' source not captured
- Line 8091: comparison_target field not implemented in engine
- Line 8108: activation_restriction action not implemented in engine
- Line 8200: Missing 'self or opponent' choice mechanism
- Line 8201: deck_bottom destination not implemented in engine
- Line 8385: select action not implemented in engine
- Line 8470: Cost should be sequential_cost (pay_energy + move_cards)
- Line 8470: Missing 'same area' destination
- Line 8487: appear action not implemented in engine
- Line 8491: Energy placement should be place_energy_under_member
- Line 8518: Missing 'self or opponent' choice mechanism
- Line 8518: Missing 'up to 2' (max) field
- Line 8522: deck_bottom destination not implemented in engine
- Line 8530: Missing multi-condition (same heart OR same cost OR same original blades)
- Line 8530: Missing 'select member' mechanism
- Line 8655: Missing 'choose card type' choice mechanism
- Line 8659: select action not implemented in engine
- Line 8698: 'only BiBi' condition needs verification (all members vs at least one)
- Line 8782: primary_effect missing actual ability structure
- Line 8779: activation_position not implemented in engine
- Line 9273: Parenthetical restriction on area appearance not implemented
- Line 9470: action_by opponent not implemented in engine
- Line 9508: Condition comparing energy counts missing comparison_type
- Line 9538-9566: Select action incomplete - missing choice mechanism

---

## ENGINE ISSUES

### Action Implementations Missing
- `restriction` action (line 8955)
- `activation_restriction` action (line 8108)
- `select` action (line 8385, 8659, 9538-9566)
- `appear` action (line 8487)
- `reveal` action (multiple lines - part of look_and_select structure)
- `place_energy_under_member` action (line 8491)
- `modify_cost` action (line 9104, 9427-9457)
- `gain_ability` action (line 9068, 9385-9426)
- `set_card_identity` action (not yet seen but referenced)
- `discard_until_count` action (not yet seen but referenced)

### Field Implementations Missing
- `comparison_target` field in condition evaluation (lines 7403, 7513, 7616, 7699, 8091, 9305, 9508, 9567)
- `destination_choice` field (line 7513)
- `same_area` destination (line 6993, 8470)
- `deck_bottom` destination (line 8201, 8522)
- `empty_area` destination (line 9277)
- `activation_position` enforcement (line 8779, 9133)
- `action_by: "opponent"` field (line 9134, 9470)
- `max` field usage for 'up to X' logic (line 8518)
- `target_member: this_member` in place_energy_under_member (line 8491)
- `parenthetical` area restriction enforcement (line 9273)

### Trigger Type Support Missing
- `each_time` trigger type (line 6854)
- `auto` trigger type (line 9152)
- Position-based triggers (left side, center, right side) (line 9133)

### Condition Logic Missing
- State transition tracking for conditions (line 9158)
- Temporal_condition appearance count tracking (line 8887)
- Either target (self OR opponent) in conditions (line 7825)
- Appearance_condition with baton touch tracking (line 9249)
- Compound condition with OR logic (line 4163)
- Multi-branch conditional based on cost results

### Mechanic Implementations Missing
- Reveal until condition loop mechanic (line 6241, 8655)
- Repeat mechanic for multi-step abilities (line 5406)
- Select and reveal from hand mechanism (line 9068)
- Cheer-revealed cards count context (line 8168)
- Full re-yell mechanic (line 7110)

### Verification Needed
- Appearance_condition position checking
- Exclude_self in condition evaluation

---

## PRIORITIZED FIX ORDER

### Phase 1: High-Impact Parser Fixes
1. Fix all look_and_select reveal action issues (lines 156, 8144, 8286, 8586, 9198, 9369)
2. Fix select_action destination issues (multiple lines)
3. Fix choice mechanism implementations (heart type/color, self or opponent)
4. Fix compound condition parsing

### Phase 2: Engine Core Implementations
1. Implement `comparison_target` field usage
2. Implement `gain_ability` action
3. Implement `modify_cost` action
4. Implement `restriction` action
5. Implement `select` action
6. Implement `appear` action
7. Implement `place_energy_under_member` action

### Phase 3: Destination Handling
1. Implement `deck_bottom` destination
2. Implement `same_area` destination
3. Implement `empty_area` destination

### Phase 4: Advanced Mechanics
1. Implement reveal until condition loop
2. Implement repeat mechanic
3. Implement state transition tracking
4. Implement baton touch tracking

### Phase 5: Trigger Types
1. Implement `auto` trigger type
2. Implement `each_time` trigger type
3. Implement position-based triggers

### Phase 6: Remaining Parser Issues
1. Fix dynamic cost calculations
2. Fix distinct name/group checks
3. Fix multi-trigger separations
4. Fix heart/blade context issues

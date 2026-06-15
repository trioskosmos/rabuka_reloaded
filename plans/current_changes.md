# Current Work: Game-feel improvements for ability UX & position selection

## What we were doing

1. **Hidden ability effects → visible**: Add `gs.rule_log.push()` to all effect types (gain_resource, set_score, etc.)
2. **Card images in selection lists**: Fix ChoiceView/ActionListView
3. **Clickable rule log entries**: LogRenderer + LogDetailModal
4. **Score transparency**: Show real score totals, not card count
5. **Step-by-step performance wizard**: `renderPerfSteps()` following rules.txt §8
6. **Fix cheats**: `exec_code` parser, add more cheats
7. **cost_total field**: Distinguish per-card cost_limit from sum-total cost_total
8. **Sequential selection**: Select 2 cards one-at-a-time from hand & discard
9. **Position choice for "stage" destination**: When placing cards on stage, player chooses which slot
10. **Multi-card + position choice**: Remaining cards auto-place after first card's position chosen
11. **Score condition evaluator fix**: Properly sum scores, not count cards
12. **Tests for all the above**

## Stopping because: Position choice for "stage" breaks 15+ existing tests.
The engine code has accumulated too many intertwined hacks that make changes fragile.
Need to refactor first, then reapply these changes properly.

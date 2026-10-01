param([string]$Tag = "base")
$suites = @(
 "condition_evaluation","parity_conditions","parity_cce_gaps","parity_hearts_constants",
 "parity_score_effects","parity_conditional_gated","parity_jidou_state","parity_baton_touch",
 "parity_card_score","parity_realcard_walkthrough","parity_small_clusters","parity_required_hearts",
 "parity_live_total","parity_live_total2","parity_under_member","parity_state_per_card",
 "parity_jidou_life","parity_yell","parity_mill_deck","parity_energy_remainder",
 "parity_position_and_source","parity_per_card","parity_per_card2","parity_per_card3",
 "parity_bp7","parity_compound_per_card","parity_seq_and_rules","parity_rules_conditions",
 "parity_rules_conditions2","parity_characterization","parity_integration",
 "parity_search_reorder","parity_wave1a","parity_wave1b","parity_wave1c","parity_jidou_move_yell"
)
$out = "C:\Users\trios\AppData\Local\Temp\kilo\suite_$Tag.txt"
"=== $Tag ===" | Out-File -Encoding ascii $out
foreach ($s in $suites) {
  $f = "tests/test_$s.c"
  if (-not (Test-Path $f)) { "$s SKIP(no file)" | Out-File -Append -Encoding ascii $out; continue }
  $exe = "rb_a_$s.exe"
  $null = & gcc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -o $exe src/ability/enums.o src/ability/types.o src/ability/vm.o src/ability/condition.o src/ability/choice.o src/ability/ability_queue.o src/ability/dynamic_count.o src/ability/util.o src/ability/cost.o src/ability/compound.o src/ability/resolver.o src/ability/effects/executor.o src/ability/effects/ability_effects.o src/ability/effects/custom.o src/ability/effects/move.o src/ability/effects/move_missing.o src/ability/effects/look.o src/ability/effects/state.o src/ability/effects/ability.o src/ability/effects/misc.o src/ability/effects/draw.o src/ability/effects/score.o src/ability/log.o src/ability/debug.o src/ability/abilities_gen.o src/game/deck_parser.o src/game/deck_builder.o src/core/card.o src/core/data.o src/core/alloc.o src/core/modifiers.o src/core/stats_pipeline.o src/core/game_state_abilities.o src/core/tracking.o src/core/zones.o src/bytecode_data.o src/core/generated/bytecode_blob.o src/core/generated/gen_data.o src/core/gen_data_rom.o src/turn/phase.o src/turn/live.o src/turn/triggers.o src/engine.o src/test_game.o $f 2>$null
  if (-not (Test-Path $exe)) { "$s BUILD_FAIL" | Out-File -Append -Encoding ascii $out; continue }
  $r = & ".\$exe" 2>&1
  $code = $LASTEXITCODE
  $fails = ($r | Select-String -Pattern "^FAIL").Count
  $total = ($r | Select-String -Pattern "^(ok:|FAIL:)").Count
  "$s exit=$code total=$total fails=$fails" | Out-File -Append -Encoding ascii $out
  Remove-Item -Force $exe -ErrorAction SilentlyContinue
}
Get-Content $out
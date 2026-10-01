param([string]$Tag = "cmp")
$suites = @(
 "condition_evaluation","parity_conditions","parity_cce_gaps","parity_hearts_constants",
 "parity_score_effects","parity_conditional_gated","parity_jidou_state",
 "parity_card_score","parity_realcard_walkthrough","parity_small_clusters","parity_required_hearts",
 "parity_live_total","parity_live_total2","parity_under_member","parity_state_per_card",
 "parity_jidou_life","parity_yell","parity_mill_deck","parity_energy_remainder",
 "parity_per_card","parity_per_card2","parity_per_card3",
 "parity_bp7","parity_compound_per_card","parity_rules_conditions",
 "parity_rules_conditions2","parity_integration",
 "parity_search_reorder","parity_wave1a","parity_wave1b","parity_wave1c","parity_jidou_move_yell"
)
$out = "C:\Users\trios\AppData\Local\Temp\kilo\cmp_$Tag.txt"
"suite|new_total|new_fail|old_total|old_fail|delta" | Out-File -Encoding ascii $out
$common = @("src/ability/enums.c","src/ability/types.c","src/ability/vm.c","src/ability/choice.c","src/ability/ability_queue.c","src/ability/dynamic_count.c","src/ability/util.c","src/ability/cost.c","src/ability/compound.c","src/ability/resolver.c","src/ability/effects/executor.c","src/ability/effects/ability_effects.c","src/ability/effects/custom.c","src/ability/effects/move.c","src/ability/effects/move_missing.c","src/ability/effects/look.c","src/ability/effects/state.c","src/ability/effects/ability.c","src/ability/effects/misc.c","src/ability/effects/draw.c","src/ability/effects/score.c","src/ability/log.c","src/ability/debug.c","src/ability/abilities_gen.c","src/game/deck_parser.c","src/game/deck_builder.c","src/core/card.c","src/core/data.c","src/core/alloc.c","src/core/modifiers.c","src/core/stats_pipeline.c","src/core/game_state_abilities.c","src/core/tracking.c","src/core/zones.c","src/bytecode_data.c","src/core/generated/bytecode_blob.c","src/core/generated/gen_data.c","src/core/gen_data_rom.c","src/turn/phase.c","src/turn/live.c","src/turn/triggers.c","src/engine.c","src/test_game.c")
foreach ($s in $suites) {
  $f = "tests/test_$s.c"
  if (-not (Test-Path $f)) { continue }
  $exe = "rb_c_$s.exe"
  $null = & gcc -std=c11 -O2 -w -Iinclude -Isrc -Isrc/core/generated -o $exe src/ability/condition.o $common $f 2>$null
  $nt=-1; $nf=-1
  if (Test-Path $exe) { $r = & ".\$exe" 2>&1; $nt = ($r | Select-String -Pattern "^(ok:|FAIL:)").Count; $nf = ($r | Select-String -Pattern "^FAIL").Count; Remove-Item -Force $exe -ErrorAction SilentlyContinue }
  $exe2 = "rb_c2_$s.exe"
  $null = & gcc -std=c11 -O2 -w -Iinclude -Isrc -Isrc/core/generated -o $exe2 "C:\Users\trios\AppData\Local\Temp\kilo\condbase\condition_base.o" $common $f 2>$null
  $ot=-1; $of=-1
  if (Test-Path $exe2) { $r2 = & ".\$exe2" 2>&1; $ot = ($r2 | Select-String -Pattern "^(ok:|FAIL:)").Count; $of = ($r2 | Select-String -Pattern "^FAIL").Count; Remove-Item -Force $exe2 -ErrorAction SilentlyContinue }
  $d = $nf - $of
  "$s|$nt|$nf|$ot|$of|$d" | Out-File -Append -Encoding ascii $out
}
Get-Content $out
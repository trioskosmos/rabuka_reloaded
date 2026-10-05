#!/bin/sh
set -e
LIBS="src/ability/enums.o src/ability/types.o src/ability/vm.o src/ability/condition.o src/ability/choice.o src/ability/ability_queue.o src/ability/dynamic_count.o src/ability/util.o src/ability/cost.o src/ability/compound.o src/ability/resolver.o src/ability/effects/executor.o src/ability/effects/ability_effects.o src/ability/effects/custom.o src/ability/effects/move.o src/ability/effects/move_missing.o src/ability/effects/rules.o src/ability/effects/look.o src/ability/effects/state.o src/ability/effects/ability.o src/ability/effects/misc.o src/ability/effects/draw.o src/ability/effects/score.o src/ability/log.o src/ability/debug.o src/ability/abilities_gen.o src/ability/describe.o src/ability/choice_renderer.o src/game/deck_parser.o src/game/deck_builder.o src/game/game_setup.o src/core/card.o src/core/data.o src/core/alloc.o src/core/game_state_core.o src/core/modifiers.o src/core/stats_pipeline.o src/core/game_state_abilities.o src/core/tracking.o src/core/zones.o src/bytecode_data.o src/core/generated/bytecode_blob.o src/core/generated/gen_data.o src/core/gen_data_rom.o src/turn/phase.o src/turn/live.o src/turn/triggers.o src/engine.o src/test_game.o src/web_server.o"
cc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -Wl,--stack,33554432 -o zz_card_verify.exe tools/zz_card_verify.c $LIBS
echo linked
./zz_card_verify.exe 2> zzs/card_verify.log || echo "RUN FAILED $?"
grep -a 'ZZCARDSUMMARY' zzs/card_verify.log || true
echo "--- first 15 mismatch lines ---"
grep -a '^ZZ' zzs/card_verify.log | grep -av ZZCARDSUMMARY | head -15 || true
echo "--- total mismatch lines ---"
grep -ac '^ZZ' zzs/card_verify.log || true
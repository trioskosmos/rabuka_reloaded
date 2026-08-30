#include "test_game.h"
#include "rabuka.h"
#include <stdio.h>

int main(void){
    if(rb_load("src")!=0){ printf("load fail\n"); return 1; }
    TestGame tg; test_game_new(&tg);
    int umi = rb_find_card_by_no("PL!-bp3-013-N");
    int filler = rb_find_card_by_no("PL!-sd1-010-SD");
    tg.state.p[0].stage[1] = umi;
    test_add_to_live(&tg, filler);
    test_add_to_live(&tg, filler);
    test_add_to_live(&tg, filler);
    int passes = 0;
    while (!rb_has_pending_choice(&tg.state) && passes < 20) {
        passes++;
        int before = tg.state.p[0].live.n;
        rb_advance_phase(&tg.state);
        printf("pass %d: phase %d -> %d  live.n %d -> %d  pending=%d\n",
               passes, before, tg.state.phase, before, tg.state.p[0].live.n, rb_has_pending_choice(&tg.state));
    }
    rb_advance_phase(&tg.state);
    rb_resume_with_choice(&tg.state, 0);
    printf("heart01_mod on umi = %d\n", rb_mods_get_heart(&tg.state.mods, umi, 1));
    return 0;
}

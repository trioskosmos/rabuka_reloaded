#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

int main(void){
    if(!rb_load("C:/Users/trios/OneDrive/Documents/rabuka_reloaded/engine_c/src")){ fprintf(stderr,"load fail\n"); return 1; }
    TestGame tg; test_game_new(&tg);
    int eli = test_id(&tg, "PL!-sd1-002-SD");
    int target_member = test_id(&tg, "PL!-sd1-001-SD");
    int new_member = test_id(&tg, "PL!-sd1-003-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    printf("eli=%d target=%d new=%d filler=%d\n", eli, target_member, new_member, filler);
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = eli;
    tg.state.p[0].stage[2] = -1;
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, new_member);
    test_give_energy(&tg, 15);
    Card c; if(rb_decode_card_by_index((uint32_t)eli,&c)){
        printf("decoded eli. has ability=%d effect=%d\n", c.ability!=NULL, (c.ability&&c.ability->effect)?1:0);
        if(c.ability){ printf("triggers=%s\n", c.ability->triggers?c.ability->triggers:"(none)"); }
        rb_free_card(&c);
    }
    int r = test_activate_ability(&tg, eli);
    printf("activate returned %d\n", r);
    printf("after activate stage[1]=%d (expect -1)\n", tg.state.p[0].stage[1]);
    printf("pending choice=%d type=%s\n", test_has_pending_choice(&tg), test_pending_choice_type(&tg));
    rb_print_state(&tg.state);
    return 0;
}

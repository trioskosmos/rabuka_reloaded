#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
static int par_hearts_total(const int h[8]){int t=0;for(int i=0;i<8;i++)t+=h[i];return t;}
int main(void) {
    if (rb_load("src") != 0) return 1;
    static TestGame game;
    test_game_new(&game);
    int live = test_id(&game, "PL!HS-bp5-022-L");
    printf("live=%d\n", live);
    int out[8];
    /* EXACTLY what the suite does: passes &game.state, not NULL */
    rb_effective_need_heart(&game.state, live, out);
    printf("with &game.state: total=%d  [", par_hearts_total(out));
    for (int k=0;k<8;k++) printf("%d ",out[k]);
    printf("]\n");
    /* and with NULL, to isolate whether the mods pointer matters */
    rb_effective_need_heart(NULL, live, out);
    printf("with NULL        : total=%d  [", par_hearts_total(out));
    for (int k=0;k<8;k++) printf("%d ",out[k]);
    printf("]\n");
    return 0;
}

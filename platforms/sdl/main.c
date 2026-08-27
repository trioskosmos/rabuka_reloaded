/* platforms/sdl/main.c — hosted reference shim
   Portable host for engine_c: window + input → RbChoice.
   Mirrors ports/3ds pattern but with SDL2/stdio. Engine never calls
   fopen/printf directly — this shim provides platform_* hooks. */
#include "rabuka.h"
#include <stdio.h>

static unsigned char *platform_read(const char *path, long *out_len){
    FILE *f=fopen(path,"rb"); if(!f) return NULL;
    fseek(f,0,SEEK_END); long n=ftell(f); fseek(f,0,SEEK_SET);
    unsigned char *b=malloc(n?n:1); if(!b){fclose(f);return NULL;}
    if(fread(b,1,n,f)!=(size_t)n){free(b);fclose(f);return NULL;}
    fclose(f); *out_len=n; return b;
}
int main(int argc,char **argv){
    const char *dir=argc>1?argv[1]:"engine_c/src";
    if(rb_load_streaming(dir, platform_read)!=0){ fprintf(stderr,"load failed\n"); return 1; }
    printf("SDL shim: loaded %u cards, %u abilities\n", rb_num_cards(), rb_num_abilities());
    GameState g; uint32_t d0[40],d1[40]; int n0=0,n1=0;
    for(uint32_t i=0;i<rb_num_cards()&&n0<40;i++) if(rb_card_ability_idx(i)!=0xFFFF) d0[n0++]=i;
    for(uint32_t i=0;i<rb_num_cards()&&n1<40;i++) if(rb_card_ability_idx(i)!=0xFFFF) d1[n1++]=i;
    rb_seed(0xCAFE); rb_game_init(&g,d0,n0,d1,n1);
    for(int t=0;t<200 && g.winner<0;t++){
        rb_turn(&g);
        while(rb_has_pending_choice(&g)){
            const RbChoice *ch=rb_get_pending_choice(&g);
            printf("[SDL] choice %d zone=%s target=%s allow_skip=%d → skip\n", ch->kind,ch->zone,ch->target,ch->allow_skip);
            rb_resume_with_choice(&g,-1);
        }
        if(t%20==0) rb_print_state(&g);
    }
    printf("done winner=%d turn=%d\n", g.winner, g.turn);
    rb_unload(); return 0;
}

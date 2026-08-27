/* platforms/cdi/cdi_main.c — bare-metal shim (SCC68070, 1 MB RAM)
   Demonstrates RB_NO_MALLOC + streaming. Real CD-i build uses m68k-elf-gcc
   and CD sector reads; this host stub proves the API compiles with
   -DRB_NO_MALLOC -ffreestanding. Engine never calls fopen — shim provides
   platform_read that streams from CD. */
#include "rabuka.h"
#include <stdio.h>
/* CD sector read stub — on hardware this reads via CD-I BIOS */
static unsigned char *cdi_read(const char *path, long *out_len){
    /* Host fallback: use host FS so the shim still runs on PC for CI */
    FILE *f=fopen(path,"rb"); if(!f) return NULL;
    fseek(f,0,SEEK_END); long n=ftell(f); fseek(f,0,SEEK_SET);
    unsigned char *b=malloc(n?n:1); if(!b){fclose(f);return NULL;}
    fread(b,1,n,f); fclose(f); *out_len=n; return b;
}
int cdi_main(void){
    /* Bare-metal entry: app at $8000, no argc/argv */
    if(rb_load_streaming("engine_c/src", cdi_read)!=0) return 1;
    GameState g; uint32_t d0[20]={0},d1[20]={0};
    for(int i=0;i<20;i++){ d0[i]=i; d1[i]=20+i; }
    rb_seed(0x1234); rb_game_init(&g,d0,20,d1,20);
    for(int t=0;t<100 && g.winner<0;t++){
        rb_turn(&g);
        while(rb_has_pending_choice(&g)) rb_resume_with_choice(&g,-1);
    }
    return g.winner;
}
#ifndef RB_NO_MALLOC
int main(void){ return cdi_main(); }
#endif

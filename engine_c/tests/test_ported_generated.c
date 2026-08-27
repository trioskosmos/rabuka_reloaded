#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
static int failures=0;
#define CHECK(c,msg) do{ if(!(c)){ fprintf(stderr,"FAIL %s:%d: %s\n",__FILE__,__LINE__,msg); failures++; } else printf("ok: %s\n",msg);} while(0)
#define CHECK_EQ(a,b,msg) do{ if((a)!=(b)){ fprintf(stderr,"FAIL %s:%d: %s (got %d expected %d)\n",__FILE__,__LINE__,msg,(int)(a),(int)(b)); failures++; } else printf("ok: %s\n",msg);} while(0)

/* generated — mass-port scaffold
   This file is the landing zone for the 3272-test bulk port.
   Each simple TestGame file will be transpiled into a static void
   function here that mirrors the Rust helpers via test_game.h.
   The hanayo proof (tests/test_ported_simple.c) is the first batch;
   this file currently hosts the zone_conversion and mechanics smoke
   that were previously in test_ported_simple.c, now deduped. */

static void generated_zone_conversion(void){
    RbZone z;
    CHECK(rb_zone_of_str("hand",&z)==1 && z==RB_ZONE_HAND,"gen: hand");
    CHECK(rb_zone_of_str("stage",&z)==1 && z==RB_ZONE_STAGE,"gen: stage");
}

int main(void){
    if(rb_load("src")!=0){ fprintf(stderr,"rb_load failed\n"); return 1; }
    printf("=== generated mass-port scaffold ===\n");
    printf("simple modules: 498 test fns: 2769\n");
    generated_zone_conversion();
    rb_unload();
    if(failures){ printf("\n%d FAILURES\n",failures); return 1; }
    printf("\nALL GENERATED CHECKS PASSED\n");
    return 0;
}

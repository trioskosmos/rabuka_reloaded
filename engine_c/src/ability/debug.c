#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdatomic.h>

static atomic_bool g_ability_debug = ATOMIC_VAR_INIT(false);

int rb_ability_debug_enabled(void){ return atomic_load(&g_ability_debug); }
void rb_ability_debug_set(int enabled){ atomic_store(&g_ability_debug, enabled); }

typedef struct {
    int indent;
} RbAbDebug;

void rb_abdebug_init(RbAbDebug *d){ if(d) d->indent=0; }
void rb_abdebug_p(RbAbDebug *d, const char *tag, const char *msg){
    if(!rb_ability_debug_enabled()) return;
    if(!d||!tag||!msg) return;
    char pad[64]={0};
    for(int i=0;i<d->indent && i<32;i++) pad[i*2]=' ', pad[i*2+1]=' ';
    fprintf(stderr,"[AB]%s%s %s\n", pad, tag, msg);
}

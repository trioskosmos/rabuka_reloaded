#include "rabuka.h"
#include <stdio.h>
int main(void){ if(rb_load("src")) return 1; int n=0;
 for(uint32_t i=0;i<rb_num_cards() && n<6;i++){ Card c; if(rb_decode_card_by_index(i,&c)){ if(rb_card_is_energy((int)i)){ printf("energy %s\n", c.card_no_idx); n++; } rb_free_card(&c);} }
 n=0; for(uint32_t i=0;i<rb_num_cards() && n<8;i++){ Card c; if(rb_decode_card_by_index(i,&c)){ if(rb_card_is_live((int)i)){ printf("live %s\n", c.card_no_idx); n++; } rb_free_card(&c);} }
 n=0; for(uint32_t i=0;i<rb_num_cards() && n<6;i++){ Card c; if(rb_decode_card_by_index(i,&c)){ if(rb_card_is_member((int)i)){ printf("member %s\n", c.card_no_idx); n++; } rb_free_card(&c);} }
 rb_unload(); return 0;}

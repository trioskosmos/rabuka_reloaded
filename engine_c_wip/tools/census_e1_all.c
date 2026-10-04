#include <stdio.h>
#include <string.h>
#include "rabuka.h"
static const char *CM = "\xe5\xb8\xb8\xe6\x99\x82";
static const char *A4[4]={"modify_required_hearts","gain_resource","modify_score","sequential"};
static long t[4], t_mem[4], t_live[4];
static int four(const char*a){for(int i=0;i<4;i++)if(a&&!strcmp(a,A4[i]))return i;return -1;}
static void walk(const AbilityEffect*e){
  if(!e||!e->action) return;
  int k=four(e->action);
  if(k>=0){t[k]++;
    if(e->target&&!strcmp(e->target,"live_total")) t_live[k]++;
  }
  for(int i=0;i<e->n_child;i++) walk(e->child[i]);
}
int main(int argc,char**argv){
  if(rb_load(argc>1?argv[1]:"src")!=0)return 2;
  long pairs=0,cp=0;
  for(int cid=0;cid<rb_num_cards();cid++){
    int mem=rb_card_is_member(cid);
    Card c; int hc=rb_decode_card_by_index((uint32_t)cid,&c);
    if(hc&&cid==291) printf("CID291 no=%s type_flags=0x%02x type_bits=%d is_member=%d score=%d\n",
        rb_card_string(c.card_no_idx), c.type_flags, c.type_flags&3, mem, c.score);
    if(hc) rb_free_card(&c);
    int n=rb_card_num_abilities((uint32_t)cid);
    for(int ai=0;ai<n;ai++){
      Ability ab; memset(&ab,0,sizeof ab);
      if(!rb_decode_card_ability((uint32_t)cid,ai,&ab))continue;
      pairs++;
      int isc=ab.triggers&&strstr(ab.triggers,CM)!=NULL;
      if(isc){cp++;
        if(ab.effect) walk(ab.effect);
        if(isc&&mem&&ab.effect){int k=four(ab.effect->action); if(k>=0) t_mem[k]++;}
      }
      rb_free_ability(&ab);
    }
  }
  long T=0,Tm=0;
  for(int i=0;i<4;i++){printf("ALLCARDS %-26s %ld  (live_total=%ld)\n",A4[i],t[i],t_live[i]);T+=t[i];printf("  member-only %-18s %ld\n",A4[i],t_mem[i]);Tm+=t_mem[i];}
  printf("PAIRS %ld CONSTANT_PAIRS %ld\n",pairs,cp);
  printf("E1_TOTAL_ALL_CARDS %ld\n",T);
  printf("E1_TOTAL_MEMBER_ONLY %ld\n",Tm);
  return 0;
}

#include <stdio.h>
#include <string.h>
#include "rabuka.h"
static const char *CM = "\xe5\xb8\xb8\xe6\x99\x82";
int main(int argc,char**argv){
  if(rb_load(argc>1?argv[1]:"src")!=0){fprintf(stderr,"load fail\n");return 2;}
  long tot=0, member=0, const_any=0;
  static char keys[512][48]; static long has[512]; int nk=0;
  for(int cid=0;cid<rb_num_cards();cid++){
    int n=rb_card_num_abilities((uint32_t)cid);
    for(int ai=0;ai<n;ai++){
      Ability ab; memset(&ab,0,sizeof ab);
      if(!rb_decode_card_ability((uint32_t)cid,ai,&ab)) continue;
      tot++;
      int isc = ab.triggers && strstr(ab.triggers,CM)!=NULL;
      if(isc){ const_any++;
        if(ab.effect&&ab.effect->action){
          const char*a=ab.effect->action;
          int found=0;
          for(int i=0;i<nk;i++) if(!strcmp(keys[i],a)){has[i]++;found=1;break;}
          if(!found&&nk<512){snprintf(keys[nk],48,"%s",a);has[nk]=1;nk++;}
          if(strstr(a,"required_hearts")||strstr(a,"need_heart")){
            member++;
            printf("NH cid=%d member=%d ab#%d action=%s trig=%s text=%.40s\n",
                   cid, rb_card_is_member(cid), ai, a, ab.triggers?ab.triggers:"-", ab.effect->text?ab.effect->text:"-");
          }
        }
      }
      rb_free_ability(&ab);
    }
  }
  printf("TOTAL_PAIRS %ld CONSTANT_PAIRS %ld NEEDHEART_MEMBERS %ld\n",tot,const_any,member);
  return 0;
}

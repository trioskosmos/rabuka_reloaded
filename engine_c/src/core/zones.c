#include "rabuka.h"
#include <string.h>
#include <stdio.h>

int rb_member_area_to_index(const char *area){
    if(!area) return -1;
    if(!strcmp(area,"left")||!strcmp(area,"left_side")||!strcmp(area,"LeftSide")) return 0;
    if(!strcmp(area,"center")||!strcmp(area,"Center")) return 1;
    if(!strcmp(area,"right")||!strcmp(area,"right_side")||!strcmp(area,"RightSide")) return 2;
    return -1;
}
const char *rb_member_area_to_str(int idx){
    if(idx==0) return "left";
    if(idx==1) return "center";
    if(idx==2) return "right";
    return "?";
}
int rb_member_area_front(int area){
    if(area==0) return 2;
    if(area==1) return 1;
    if(area==2) return 0;
    return -1;
}

int rb_check_trigger_position(const char *triggers, int card_position){
    if(!triggers) return 1;
    if(strstr(triggers,"左サイド") && card_position!=0) return 0;
    if(strstr(triggers,"右サイド") && card_position!=2) return 0;
    if(strstr(triggers,"センター") && card_position!=1) return 0;
    return 1;
}

int rb_check_effect_position(const char *effect_pos, int card_position){
    if(!effect_pos) return 1;
    if(strchr(effect_pos,',')){
        char buf[64]; strncpy(buf, effect_pos, 63); buf[63]=0;
        char *tok = strtok(buf, ",");
        while(tok){
            while(*tok==' ') tok++;
            char *end = tok+strlen(tok)-1; while(end>tok && *end==' ') *end--=0;
            if((!strcmp(tok,"center")||!strcmp(tok,"中央")) && card_position==1) return 1;
            if((!strcmp(tok,"left")||!strcmp(tok,"左")||!strcmp(tok,"左側")||!strcmp(tok,"left_side")) && card_position==0) return 1;
            if((!strcmp(tok,"right")||!strcmp(tok,"右")||!strcmp(tok,"右側")||!strcmp(tok,"right_side")) && card_position==2) return 1;
            tok = strtok(NULL, ",");
        }
        return 0;
    }
    if((!strcmp(effect_pos,"center")||!strcmp(effect_pos,"中央")) && card_position==1) return 1;
    if((!strcmp(effect_pos,"left")||!strcmp(effect_pos,"左")||!strcmp(effect_pos,"左側")||!strcmp(effect_pos,"left_side")) && card_position==0) return 1;
    if((!strcmp(effect_pos,"right")||!strcmp(effect_pos,"右")||!strcmp(effect_pos,"右側")||!strcmp(effect_pos,"right_side")) && card_position==2) return 1;
    if(!strcmp(effect_pos,"center")||!strcmp(effect_pos,"left")||!strcmp(effect_pos,"right")||
       !strcmp(effect_pos,"左")||!strcmp(effect_pos,"右")||!strcmp(effect_pos,"中央")||
       !strcmp(effect_pos,"左側")||!strcmp(effect_pos,"右側")||!strcmp(effect_pos,"left_side")||!strcmp(effect_pos,"right_side"))
        return 0;
    return 1;
}

int rb_stage_get_area(const int stage[RB_STAGE_SIZE], int area){
    if(area<0||area>=RB_STAGE_SIZE) return RB_EMPTY_SLOT;
    return stage[area];
}
void rb_stage_set_area(int stage[RB_STAGE_SIZE], int area, int card_id){
    if(area<0||area>=RB_STAGE_SIZE) return;
    stage[area]=card_id;
}
int rb_stage_position_change(int stage[RB_STAGE_SIZE], int from_area, int to_area){
    if(from_area==to_area) return -1;
    if(from_area<0||from_area>=RB_STAGE_SIZE||to_area<0||to_area>=RB_STAGE_SIZE) return -1;
    int card_id = stage[from_area];
    if(card_id==RB_EMPTY_SLOT) return -1;
    int dest = stage[to_area];
    if(dest!=RB_EMPTY_SLOT){
        stage[from_area]=dest;
        stage[to_area]=card_id;
    } else {
        stage[to_area]=card_id;
        stage[from_area]=RB_EMPTY_SLOT;
    }
    return card_id;
}

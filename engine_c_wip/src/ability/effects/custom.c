#include "rabuka.h"

#include <string.h>

int rb_execute_custom_effect(GameState *g, int actor, AbilityEffect *effect,
                             const char *action_str)
{
    if (!g || !effect) return 0;
    return rb_translated_execute_custom_effect(g, actor, effect, action_str);
}

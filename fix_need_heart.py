with open('engine_c/src/ability/effects/state.c', 'r') as f:
    content = f.read()

old = """/* Mirror Card::need_heart_satisfied — delegates to check_heart_requirement. */
int rb_card_need_heart_satisfied(int card_id, const int *need, const int *provided) {
    (void)card_id;
    return rb_check_heart_requirement(need, provided);
}"""

new = """/* Mirror Card::need_heart_satisfied — delegates to check_heart_requirement. */
int rb_card_need_heart_satisfied(const Card *c, const int *need, const int *provided) {
    (void)c;
    return rb_check_heart_requirement(need, provided);
}"""

content = content.replace(old, new)

with open('engine_c/src/ability/effects/state.c', 'w') as f:
    f.write(content)
print('Done')
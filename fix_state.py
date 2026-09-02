with open('engine_c/src/ability/effects/state.c', 'r') as f:
    content = f.read()

old = """/* Mirror HeartColor::from_str / parse_heart_color. */
int rb_parse_heart_color(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "heart00") || !strcmp(s, "h00") || !strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return 0;
    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return 1;
    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return 2;
    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return 3;
    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return 4;
    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return 5;
    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return 6;
    if (!strcmp(s, "all") || !strcmp(s, "b_all")) return 7;
    if (strncmp(s, "b_", 2) == 0) return rb_parse_heart_color(s + 2);
    return 0;
}"""

new = """/* Mirror HeartColor::from_str / parse_heart_color. */
RbHeartColor rb_parse_heart_color(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "heart00") || !strcmp(s, "h00") || !strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return 0;
    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return 1;
    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return 2;
    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return 3;
    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return 4;
    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return 5;
    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return 6;
    if (!strcmp(s, "all") || !strcmp(s, "b_all")) return 7;
    if (strncmp(s, "b_", 2) == 0) return rb_parse_heart_color(s + 2);
    return 0;
}"""

content = content.replace(old, new)

with open('engine_c/src/ability/effects/state.c', 'w') as f:
    f.write(content)
print('Done')
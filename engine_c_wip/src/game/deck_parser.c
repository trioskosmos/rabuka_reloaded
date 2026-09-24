#include "deck_parser.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char *dup_range(const char *start, size_t length)
{
    char *result = (char *)malloc(length + 1u);
    if (!result) return NULL;
    memcpy(result, start, length);
    result[length] = '\0';
    return result;
}

static char *trim_copy(const char *start, size_t length)
{
    while (length > 0 && isspace((unsigned char)*start)) {
        start++;
        length--;
    }
    while (length > 0 && isspace((unsigned char)start[length - 1])) length--;
    return dup_range(start, length);
}

int rb_normalize_card_no(const char *raw, char *out, size_t out_size)
{
    size_t length;
    if (!raw || !out || out_size == 0) return -1;
    while (*raw == ' ' || *raw == '\t') raw++;
    length = strlen(raw);
    while (length > 0 && isspace((unsigned char)raw[length - 1])) length--;
    if (length == 0 || length >= out_size) return -1;
    memcpy(out, raw, length);
    out[length] = '\0';
    while (length >= 3 && (unsigned char)out[length - 3] == 0xef &&
           (unsigned char)out[length - 2] == 0xbc &&
           ((unsigned char)out[length - 1] == 0x9b || (unsigned char)out[length - 1] == 0x81)) {
        length -= 3;
    }
    out[length] = '\0';
    return 0;
}

static int parse_quantity(const char *text, uint8_t *quantity)
{
    char *end = NULL;
    long value = strtol(text, &end, 10);
    if (end == text || value <= 0 || value > 255) return 0;
    while (*end && isspace((unsigned char)*end)) end++;
    if (*end) return 0;
    *quantity = (uint8_t)value;
    return 1;
}

int rb_parse_deck_line(const char *line, char *card_no, size_t card_no_size, uint8_t *quantity)
{
    const char *marker;
    char *left;
    char *right;
    uint8_t parsed_quantity = 1;
    int quantity_first = 0;
    if (!line || !card_no || !quantity || card_no_size == 0) return 0;
    while (*line && isspace((unsigned char)*line)) line++;
    if (!*line || *line == '#' || (line[0] == '/' && line[1] == '/')) return 0;
    marker = strstr(line, " x ");
    if (!marker) marker = strstr(line, " X ");
    if (marker) {
        left = trim_copy(line, (size_t)(marker - line));
        right = trim_copy(marker + 3, strlen(marker + 3));
        if (!left || !right) {
            free(left);
            free(right);
            return 0;
        }
        quantity_first = parse_quantity(left, &parsed_quantity);
        if (!quantity_first && !parse_quantity(right, &parsed_quantity)) {
            free(left);
            free(right);
            return 0;
        }
        if (rb_normalize_card_no(quantity_first ? right : left,
                                 card_no, card_no_size) != 0 || !strchr(card_no, '-')) {
            free(left);
            free(right);
            return 0;
        }
        free(left);
        free(right);
        *quantity = parsed_quantity;
        return 1;
    }
    if (strchr(line, '-') && !strchr(line, ' ') && !strchr(line, '\t')) {
        if (rb_normalize_card_no(line, card_no, card_no_size) != 0) return 0;
        *quantity = 1;
        return 1;
    }
    return 0;
}

static char *strip_html(const char *content)
{
    size_t length = strlen(content);
    char *result = (char *)malloc(length + 1u);
    size_t input = 0;
    size_t output = 0;
    int in_tag = 0;
    if (!result) return NULL;
    while (input < length) {
        char c = content[input++];
        if (c == '<') {
            in_tag = 1;
            if (output > 0 && result[output - 1] != '\n') result[output++] = '\n';
            continue;
        }
        if (c == '>') {
            in_tag = 0;
            continue;
        }
        if (!in_tag) {
            if (c == '\r') c = '\n';
            if (c == '\n' && output > 0 && result[output - 1] == '\n') continue;
            result[output++] = c;
        }
    }
    result[output] = '\0';
    return result;
}

int rb_parse_deck_content(const char *content, char ***card_numbers, size_t *card_count)
{
    char *cleaned;
    char *line;
    char **numbers = NULL;
    size_t count = 0;
    size_t capacity = 0;
    if (!content || !card_numbers || !card_count) return -1;
    cleaned = strip_html(content);
    if (!cleaned) return -1;
    line = strtok(cleaned, "\n");
    while (line != NULL) {
        char card_no[128];
        uint8_t quantity;
        if (rb_parse_deck_line(line, card_no, sizeof(card_no), &quantity)) {
            for (uint8_t copy = 0; copy < quantity; copy++) {
            if (count == capacity) {
                size_t next = capacity ? capacity * 2u : 32u;
                char **grown = (char **)realloc(numbers, next * sizeof(*grown));
                if (!grown) {
                    for (size_t i = 0; i < count; i++) free(numbers[i]);
                    free(numbers);
                    free(cleaned);
                    return -1;
                }
                numbers = grown;
                capacity = next;
            }
            numbers[count] = dup_range(card_no, strlen(card_no));
            if (!numbers[count]) {
                for (size_t i = 0; i < count; i++) free(numbers[i]);
                free(numbers);
                free(cleaned);
                return -1;
            }
            count++;
            }
        }
        line = strtok(NULL, "\n");
    }
    free(cleaned);
    *card_numbers = numbers;
    *card_count = count;
    return 0;
}

int rb_parse_deck_file(const char *path, RbDeckList *out)
{
    FILE *file;
    long length;
    char *content;
    char **numbers = NULL;
    size_t count = 0;
    if (!path || !out) return -1;
    file = fopen(path, "rb");
    if (!file) return -1;
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        return -1;
    }
    length = ftell(file);
    if (length < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        return -1;
    }
    content = (char *)malloc((size_t)length + 1u);
    if (!content) {
        fclose(file);
        return -1;
    }
    if (fread(content, 1, (size_t)length, file) != (size_t)length) {
        free(content);
        fclose(file);
        return -1;
    }
    content[length] = '\0';
    fclose(file);
    if (rb_parse_deck_content(content, &numbers, &count) != 0) {
        free(content);
        return -1;
    }
    out->entries = (RbDeckEntry *)calloc(count ? count : 1u, sizeof(*out->entries));
    if (!out->entries) {
        for (size_t i = 0; i < count; i++) free(numbers[i]);
        free(numbers);
        free(content);
        return -1;
    }
    out->count = 0;
    for (size_t i = 0; i < count; i++) {
        int duplicate = 0;
        for (size_t j = 0; j < out->count; j++) {
            if (!strcmp(out->entries[j].card_no, numbers[i])) {
                if (out->entries[j].quantity < 255) out->entries[j].quantity++;
                duplicate = 1;
                break;
            }
        }
        if (!duplicate) {
            out->entries[out->count].card_no = numbers[i];
            out->entries[out->count].quantity = 1;
            out->count++;
        } else {
            free(numbers[i]);
        }
    }
    free(numbers);
    free(content);
    const char *name = strrchr(path, '/');
    const char *backslash = strrchr(path, '\\');
    if (backslash > name) name = backslash;
    out->name = dup_range(name ? name + 1 : path, strlen(name ? name + 1 : path));
    return out->name ? 0 : -1;
}

void rb_deck_list_free(RbDeckList *deck)
{
    if (!deck) return;
    for (size_t i = 0; i < deck->count; i++) free(deck->entries[i].card_no);
    free(deck->entries);
    free(deck->name);
    memset(deck, 0, sizeof(*deck));
}

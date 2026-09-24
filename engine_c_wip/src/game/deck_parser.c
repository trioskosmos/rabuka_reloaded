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
    const char *start;
    size_t length;
    if (!raw || !out || out_size == 0) return -1;
    while (*raw && isspace((unsigned char)*raw)) raw++;
    start = raw;
    while (*raw && !isspace((unsigned char)*raw)) raw++;
    length = (size_t)(raw - start);
    if (length == 0 || length >= out_size) return -1;
    memcpy(out, start, length);
    out[length] = '\0';
    while (length > 0 && (out[length - 1] == '+' || out[length - 1] == '!')) length--;
    while (length >= 3 && (unsigned char)out[length - 3] == 0xef &&
           (unsigned char)out[length - 2] == 0xbc &&
           ((unsigned char)out[length - 1] == 0x8b ||
            (unsigned char)out[length - 1] == 0x81 ||
            (unsigned char)out[length - 1] == 0x9b)) {
        length -= 3;
    }
    out[length] = '\0';
    return length > 0 ? 0 : -1;
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

static int is_deck_separator(const char *p, size_t *width)
{
    if (!p) return 0;
    if (*p == 'x' || *p == 'X') {
        if (width) *width = 1;
        return 1;
    }
    if ((unsigned char)p[0] == 0xc3 && (unsigned char)p[1] == 0x97) {
        if (width) *width = 2;
        return 1;
    }
    return 0;
}

static void strip_inline_comment(char *line)
{
    if (!line) return;
    char *comment = strstr(line, "//");
    if (!comment) comment = strchr(line, '#');
    if (comment) *comment = '\0';
}

int rb_parse_deck_line(const char *line, char *card_no, size_t card_no_size, uint8_t *quantity)
{
    char *work;
    char *left;
    char *right;
    if (!line || !card_no || !quantity || card_no_size == 0) return 0;
    work = trim_copy(line, strlen(line));
    if (!work) return 0;
    strip_inline_comment(work);
    char *trimmed = trim_copy(work, strlen(work));
    free(work);
    if (!trimmed) return 0;
    work = trimmed;
    size_t length = strlen(work);
    if (length == 0 || work[0] == '#') {
        free(work);
        return 0;
    }

    for (size_t i = 0; i < length; i++) {
        size_t width = 0;
        if (!is_deck_separator(work + i, &width)) continue;
        left = trim_copy(work, i);
        right = trim_copy(work + i + width, length - i - width);
        if (!left || !right) {
            free(left);
            free(right);
            free(work);
            return 0;
        }
        uint8_t parsed_quantity = 0;
        int quantity_first = parse_quantity(left, &parsed_quantity);
        int quantity_second = parse_quantity(right, &parsed_quantity);
        if ((quantity_first || quantity_second) &&
            rb_normalize_card_no(quantity_first ? right : left, card_no, card_no_size) == 0 &&
            strchr(card_no, '-')) {
            free(left);
            free(right);
            free(work);
            *quantity = parsed_quantity;
            return 1;
        }
        free(left);
        free(right);
        i += width - 1;
    }

    if (strchr(work, '-') && !strchr(work, ' ') && !strchr(work, '\t')) {
        int result = rb_normalize_card_no(work, card_no, card_no_size);
        free(work);
        if (result != 0) return 0;
        *quantity = 1;
        return 1;
    }
    free(work);
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
    if (!card_numbers || !card_count) return -1;
    *card_numbers = NULL;
    *card_count = 0;
    if (!content) return -1;
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

static void rb_deck_set_error(char *error, size_t error_size, const char *message)
{
    if (!error || error_size == 0) return;
    snprintf(error, error_size, "%s", message ? message : "deck error");
}

static int rb_deck_append_entry(RbDeckList *deck, const char *card_no, uint8_t quantity)
{
    if (!deck || !card_no || quantity == 0) return 0;
    if (deck->count == SIZE_MAX / sizeof(*deck->entries) - 1) return 0;
    RbDeckEntry *entries = (RbDeckEntry *)realloc(
        deck->entries, (deck->count + 1) * sizeof(*deck->entries));
    if (!entries) return 0;
    deck->entries = entries;
    deck->entries[deck->count].card_no = dup_range(card_no, strlen(card_no));
    if (!deck->entries[deck->count].card_no) return 0;
    deck->entries[deck->count].quantity = quantity;
    deck->count++;
    return 1;
}

static char *rb_deck_basename_without_txt(const char *path)
{
    if (!path) return NULL;
    const char *base = path;
    for (const char *p = path; *p; p++) {
        if (*p == '/' || *p == '\\') base = p + 1;
    }
    size_t length = strlen(base);
    if (length >= 4) {
        const char *suffix = base + length - 4;
        if ((suffix[0] == '.' && suffix[1] == 't' && suffix[2] == 'x' && suffix[3] == 't') ||
            (suffix[0] == '.' && suffix[1] == 'T' && suffix[2] == 'X' && suffix[3] == 'T'))
            length -= 4;
    }
    return dup_range(base, length);
}

RbDeckStatus rb_parse_deck_file_ex(const char *path, RbDeckList *out,
                                   char *error, size_t error_size)
{
    if (error && error_size) error[0] = '\0';
    if (!path || !out) {
        rb_deck_set_error(error, error_size, "invalid deck arguments");
        return RB_DECK_EINVAL;
    }
    memset(out, 0, sizeof(*out));
    FILE *file = fopen(path, "rb");
    if (!file) {
        rb_deck_set_error(error, error_size, "failed to read deck file");
        return RB_DECK_EIO;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        rb_deck_set_error(error, error_size, "failed to seek deck file");
        return RB_DECK_EIO;
    }
    long length = ftell(file);
    if (length < 0 || fseek(file, 0, SEEK_SET) != 0) {
        fclose(file);
        rb_deck_set_error(error, error_size, "failed to measure deck file");
        return RB_DECK_EIO;
    }
    char *content = (char *)malloc((size_t)length + 1u);
    if (!content) {
        fclose(file);
        rb_deck_set_error(error, error_size, "out of memory reading deck file");
        return RB_DECK_ENOMEM;
    }
    size_t read = fread(content, 1, (size_t)length, file);
    fclose(file);
    if (read != (size_t)length) {
        free(content);
        rb_deck_set_error(error, error_size, "short read of deck file");
        return RB_DECK_EIO;
    }
    content[length] = '\0';
    char *cleaned = strip_html(content);
    free(content);
    if (!cleaned) {
        rb_deck_set_error(error, error_size, "out of memory parsing deck file");
        return RB_DECK_ENOMEM;
    }
    char *line = strtok(cleaned, "\r\n");
    while (line) {
        char card_no[128];
        uint8_t quantity = 0;
        if (rb_parse_deck_line(line, card_no, sizeof(card_no), &quantity) &&
            !rb_deck_append_entry(out, card_no, quantity)) {
            free(cleaned);
            rb_deck_list_free(out);
            rb_deck_set_error(error, error_size, "deck entry allocation failed");
            return RB_DECK_ENOMEM;
        }
        line = strtok(NULL, "\r\n");
    }
    free(cleaned);
    out->name = rb_deck_basename_without_txt(path);
    if (!out->name) {
        rb_deck_list_free(out);
        rb_deck_set_error(error, error_size, "out of memory naming deck");
        return RB_DECK_ENOMEM;
    }
    return RB_DECK_OK;
}

int rb_parse_deck_file(const char *path, RbDeckList *out)
{
    return rb_parse_deck_file_ex(path, out, NULL, 0) == RB_DECK_OK ? 0 : -1;
}

const char *rb_deck_status_str(RbDeckStatus status)
{
    switch (status) {
        case RB_DECK_OK: return "ok";
        case RB_DECK_EINVAL: return "invalid argument";
        case RB_DECK_EIO: return "deck I/O error";
        case RB_DECK_EFORMAT: return "deck format error";
        case RB_DECK_EOVERFLOW: return "deck overflow";
        case RB_DECK_ENOMEM: return "out of memory";
        default: return "unknown deck error";
    }
}

int rb_deck_list_to_card_numbers(const RbDeckList *deck, char ***out, size_t *out_count)
{
    if (!out || !out_count) return -1;
    *out = NULL;
    *out_count = 0;
    if (!deck) return -1;
    size_t total = 0;
    for (size_t i = 0; i < deck->count; i++) {
        if (deck->entries[i].quantity == 0) return -1;
        if (total > SIZE_MAX - deck->entries[i].quantity) return -1;
        total += deck->entries[i].quantity;
    }
    if (total == 0) return 0;
    char **numbers = (char **)calloc(total, sizeof(*numbers));
    if (!numbers) return -1;
    size_t count = 0;
    for (size_t i = 0; i < deck->count; i++) {
        for (uint8_t copy = 0; copy < deck->entries[i].quantity; copy++) {
            numbers[count] = dup_range(deck->entries[i].card_no,
                                       strlen(deck->entries[i].card_no));
            if (!numbers[count]) {
                rb_deck_card_numbers_free(numbers, count);
                return -1;
            }
            count++;
        }
    }
    *out = numbers;
    *out_count = count;
    return 0;
}

void rb_deck_card_numbers_free(char **cards, size_t count)
{
    if (!cards) return;
    for (size_t i = 0; i < count; i++) free(cards[i]);
    free(cards);
}

void rb_deck_list_free(RbDeckList *deck)
{
    if (!deck) return;
    for (size_t i = 0; i < deck->count; i++) free(deck->entries[i].card_no);
    free(deck->entries);
    free(deck->name);
    memset(deck, 0, sizeof(*deck));
}

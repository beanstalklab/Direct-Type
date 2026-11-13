/*
 * C smoke test for the ABI in `viet.h`: the same functions the fcitx5 addon will call,
 * compiled by `cc` and linked against `libviet.a` (and again against `libviet.so`).
 * `scripts/c_smoke_test.sh` builds, links and runs it.
 *
 * What this file proves that `cargo test` cannot: the header and the Rust implementation
 * agree on the struct layout, the enum values, the byte lengths and the pointer lifetimes,
 * from a caller that is not Rust.
 *
 * The panic path is not here on purpose. It is injected in the Rust build only, so a C
 * caller cannot reach it; `crates/viet-ffi/src/lib.rs` tests it.
 */

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "viet.h"

static int failures = 0;

static void check(int condition, const char *message) {
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", message);
        failures += 1;
    }
}

static viet_input_t character(char key) {
    viet_input_t input;
    input.kind = 0;
    input.code_point = (uint32_t)(unsigned char)key;
    return input;
}

static viet_input_t event(int32_t kind) {
    viet_input_t input;
    input.kind = kind;
    input.code_point = 0;
    return input;
}

/* Delete `count` UTF-8 characters from the end of `text`.
 *
 * The engine counts deletions in characters, not bytes: `ệ` is three bytes, so a frontend
 * that popped `backspaces` bytes would corrupt the buffer. This helper walks back over
 * continuation bytes until it has removed whole characters, which is what the addon has to
 * do as well.
 */
static void delete_characters(char *text, size_t *length, uint16_t count) {
    while (count > 0 && *length > 0) {
        size_t index = *length - 1;
        while (index > 0 && ((unsigned char)text[index] & 0xC0) == 0x80) {
            index -= 1;
        }
        *length = index;
        count -= 1;
    }
    text[*length] = '\0';
}

/* Apply one action to `text` the way an input method frontend does. */
static void perform(char *text, size_t *length, size_t capacity,
                    const viet_action_t *action, const char *commit) {
    check(*length + (size_t)action->commit_length < capacity, "the buffer is large enough");
    delete_characters(text, length, action->backspaces);
    if (action->commit_length > 0) {
        memcpy(text + *length, commit, action->commit_length);
    }
    *length += (size_t)action->commit_length;
    text[*length] = '\0';
}

/* Type a word and end it, writing the text the application ends up with into `text`. */
static void type_keys(viet_engine_t *engine, const char *keys, char *text, size_t capacity) {
    viet_action_t action;
    const char *commit = NULL;
    size_t length = 0;
    text[0] = '\0';

    for (const char *key = keys; *key != '\0'; key += 1) {
        viet_input_t input = character(*key);
        viet_engine_feed(engine, &input, &action, &commit, NULL);
        check(action.consumed == 1, "a typed key is consumed");
        perform(text, &length, capacity, &action, commit);
    }

    viet_input_t boundary = event(2);
    viet_engine_feed(engine, &boundary, &action, &commit, NULL);
    check(action.consumed == 0, "the boundary key is forwarded");
    check(action.boundary == 1, "the boundary is reported");
    perform(text, &length, capacity, &action, commit);
}

int main(void) {
    char text[64];

    viet_config_t configuration;
    memset(&configuration, 0, sizeof configuration);
    configuration.method = 0;     /* telex */
    configuration.tone_style = 1; /* new */
    configuration.auto_restore = 1;
    configuration.spell_check = 0;
    configuration.free_tone_mark = 1;

    viet_engine_t *engine = viet_engine_new(&configuration);
    check(engine != NULL, "an engine is created from an explicit configuration");
    if (engine == NULL) {
        return 1;
    }

    /* `việt`: the ệ is three bytes of UTF-8, so a caller that trusted strlen would be
       wrong about the span. */
    type_keys(engine, "vieejt", text, sizeof text);
    check(strcmp(text, "vi\xE1\xBB\x87" "t") == 0, "vieejt is viet with a dot below");

    type_keys(engine, "dduowngf", text, sizeof text);
    check(strcmp(text, "\xC4\x91\xC6\xB0\xE1\xBB\x9D" "ng") == 0, "dduowngf is duong");

    type_keys(engine, "uowng", text, sizeof text);
    check(strcmp(text, "\xC6\xB0\xC6\xA1" "ng") == 0, "uowng is uong with horns");

    type_keys(engine, "asd", text, sizeof text);
    check(strcmp(text, "asd") == 0, "auto_restore gives the keystrokes back");

    type_keys(engine, "vieejt", text, sizeof text);
    check(strcmp(text, "vi\xE1\xBB\x87" "t") == 0, "the engine is reusable after a word");

    /* The same word through VNI, from a second engine. */
    configuration.method = 2;
    viet_engine_t *vni = viet_engine_new(&configuration);
    check(vni != NULL, "a VNI engine is created");
    if (vni != NULL) {
        type_keys(vni, "d9u7o7ng2", text, sizeof text);
        check(strcmp(text, "\xC4\x91\xC6\xB0\xE1\xBB\x9D" "ng") == 0, "VNI reaches duong");
    }

    /* A configuration can be changed at runtime: the same engine, another rule pack. */
    if (vni != NULL) {
        configuration.method = 0;
        viet_engine_set_config(vni, &configuration);
        type_keys(vni, "vieejt", text, sizeof text);
        check(strcmp(text, "vi\xE1\xBB\x87" "t") == 0,
              "an engine can switch back to Telex");
        viet_engine_set_config(vni, NULL);
        type_keys(vni, "uowng", text, sizeof text);
        check(strcmp(text, "\xC6\xB0\xC6\xA1" "ng") == 0,
              "a NULL configuration applies the defaults");
    }

    /* A NULL configuration means the defaults, as the header promises. */
    viet_engine_t *defaults = viet_engine_new(NULL);
    check(defaults != NULL, "an engine is created from NULL");
    if (defaults != NULL) {
        type_keys(defaults, "hoaf", text, sizeof text);
        check(strcmp(text, "h\xC3\xB2" "a") == 0, "the default tone style is new");
        viet_engine_reset(defaults);
    }

    /* A NULL engine is a no-op rather than a crash: a frontend that has already torn its
       engine down must be able to keep calling. */
    viet_action_t action;
    const char *commit = NULL;
    viet_input_t input = character('a');
    viet_engine_feed(NULL, &input, &action, &commit, NULL);
    check(action.consumed == 0, "a NULL engine reports a no-op");
    check(action.commit_length == 0, "a NULL engine commits nothing");
    check(commit == NULL, "a NULL engine hands out no string");

    viet_engine_free(vni);
    viet_engine_free(defaults);
    viet_engine_free(engine);
    viet_engine_free(NULL);

    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("C ABI smoke test passed\n");
    return 0;
}

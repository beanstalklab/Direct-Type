/*
 * C ABI for the Vietnamese transformation engine.
 *
 * Contract (docs/SPEC-engine-core.md §2.2):
 *  - Strings are borrowed from the engine and are valid only until the next
 *    viet_engine_feed() call on the same engine.
 *  - Strings are length-delimited in **bytes** of UTF-8 and are **not** NUL-terminated:
 *    read commit_length bytes, and do not call strlen(). A length of zero means the
 *    pointer must not be dereferenced.
 *  - No allocation or ownership crosses this boundary.
 *  - A panic inside the engine is caught and reported as a no-op action.
 */

#ifndef VIET_H
#define VIET_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct viet_engine viet_engine_t;

/* method: 0 = telex, 1 = simple_telex, 2 = vni */
/* tone_style: 0 = old, 1 = new */
typedef struct {
  int32_t method;
  int32_t tone_style;
  int32_t auto_restore;
  int32_t spell_check;
  int32_t free_tone_mark;
} viet_config_t;

/* kind: 0 = char, 1 = backspace, 2 = boundary, 3 = reset */
typedef struct {
  int32_t  kind;
  uint32_t code_point; /* Unicode scalar value when kind == 0 */
} viet_input_t;

typedef struct {
  int32_t  consumed;      /* 1 = the key must not be forwarded */
  uint16_t backspaces;    /* characters (not bytes) to delete before committing */
  uint32_t commit_length; /* bytes of UTF-8 in the commit span */
  uint32_t preedit_length;/* bytes of UTF-8; always 0 in v1, there is no buffer */
  int32_t  boundary;      /* 1 = a word just ended */
  uint32_t reserved_flags;/* must be 0 in v1 */
} viet_action_t;

viet_engine_t *viet_engine_new(const viet_config_t *configuration);
void           viet_engine_free(viet_engine_t *engine);
void           viet_engine_reset(viet_engine_t *engine);

/* out_action and both string spans point into engine-owned memory, valid until
   the next viet_engine_feed call. Output pointers may be NULL if unused. */
void viet_engine_feed(viet_engine_t *engine, const viet_input_t *input,
                      viet_action_t *out_action,
                      const char **out_commit,
                      const char **out_preedit);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* VIET_H */

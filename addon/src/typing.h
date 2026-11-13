#ifndef DIRECTTYPE_TYPING_H
#define DIRECTTYPE_TYPING_H

#include <cstdint>

#include "keys.h"
#include "text_target.h"
#include "viet.h"

namespace directtype {

/// Feed one classified key through the engine and apply what it asks for: delete the
/// characters it gives back, then insert what it commits. Returns whether the key was
/// consumed, i.e. whether the application must not also receive it.
///
/// This is the whole typing path of the addon. It knows nothing about fcitx5 on purpose,
/// so the same code runs behind an `InputContext` and behind a test buffer.
bool submit(viet_engine_t *engine, KeyAction action, uint32_t codepoint,
            TextTarget *target);

} // namespace directtype

#endif // DIRECTTYPE_TYPING_H

#ifndef DIRECTTYPE_TYPING_H
#define DIRECTTYPE_TYPING_H

#include <cstdint>

#include "keys.h"
#include "text_target.h"
#include "viet.h"

namespace directtype {

/// The addon's typing state: the Vietnamese engine and whether it is switched on.
///
/// This is the whole behaviour of the addon apart from fcitx5 — a key either passes through
/// or goes to the engine — and it owns no fcitx5 type, so it is what the tests drive. The
/// fcitx5 adapter in `engine.cpp` classifies keys and forwards what this does not consume.
class TypingSession {
public:
    explicit TypingSession(const viet_config_t &configuration);
    ~TypingSession();

    TypingSession(const TypingSession &) = delete;
    TypingSession &operator=(const TypingSession &) = delete;

    bool vietnameseEnabled() const { return vietnameseEnabled_; }

    /// Drop the word in progress without changing the language.
    void reset();

    /// Apply a new engine configuration. The word in progress is dropped by the engine: a
    /// settings change is not a typing event, and the text on screen was derived with the
    /// previous rule pack.
    void setEngineConfiguration(const viet_config_t &configuration);

    /// Switch the language. The word in progress is dropped rather than rewritten: the text
    /// on screen belongs to the application, and a toggle must never edit it. The next key
    /// starts a fresh word, so nothing from the previous one leaks across the switch.
    void setVietnameseEnabled(bool enabled);

    /// Feed one classified key. Returns whether it was consumed; with Vietnamese off
    /// nothing is consumed, so the application types the key itself.
    bool submit(KeyAction action, uint32_t codepoint, TextTarget *target);

private:
    viet_engine_t *engine_;
    bool vietnameseEnabled_;
};

} // namespace directtype

#endif // DIRECTTYPE_TYPING_H

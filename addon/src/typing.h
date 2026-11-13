#ifndef DIRECTTYPE_TYPING_H
#define DIRECTTYPE_TYPING_H

#include <cstdint>
#include <string>

#include "keys.h"
#include "text_target.h"
#include "viet.h"

namespace directtype {

/// What one keystroke means for the application.
struct TypingResult {
    /// The key belongs to the input method and must not reach the application.
    bool consumed = false;
    /// The word finished here: `text` is what to commit, and the session is empty again.
    bool ended = false;
    /// The word to show in the preedit, or the finished word when `ended` is true.
    std::string text;
};

/// The addon's typing state: the Vietnamese engine, the word it is building, and whether
/// Vietnamese is switched on.
///
/// The word is held here, not in the application. Each keystroke goes to the engine, whose
/// answer is a deletion-and-insert against the text it has shown so far; that text is this
/// buffer, so the same engine works whether the frontend rewrites committed text or keeps a
/// preedit. It owns no fcitx5 type, so it is what the tests drive.
class TypingSession {
public:
    explicit TypingSession(const viet_config_t &configuration);
    ~TypingSession();

    TypingSession(const TypingSession &) = delete;
    TypingSession &operator=(const TypingSession &) = delete;

    bool vietnameseEnabled() const { return vietnameseEnabled_; }

    /// Whether a word is being built, i.e. whether there is a preedit to clear.
    bool hasWord() const { return !word_.empty(); }

    /// Take the word out of the session, for a caller that has to finish it for a reason of
    /// its own (a language toggle, a configuration change, a lost focus).
    std::string takeWord();

    /// Feed one key. A boundary ends the word: `ended` is set and `text` is what to commit.
    /// Backspace deletes the last keystroke from the word rather than from the application.
    TypingResult type(KeyAction action, uint32_t codepoint);

    /// Switch the language. The caller finishes the word first (with `takeWord` and a
    /// commit); the session drops the engine's copy either way, so the next keystroke starts
    /// a new word instead of continuing the one that was just committed.
    void setVietnameseEnabled(bool enabled);

    /// Apply a new engine configuration. The word in progress has to be finished by the
    /// caller first: the engine drops it, because it was derived with the previous rules.
    void setEngineConfiguration(const viet_config_t &configuration);

    /// Drop everything, including the word.
    void reset();

private:
    /// Feed one input and apply its deletion-and-insert to `word_`.
    void feed(const viet_input_t &input);

    viet_engine_t *engine_;
    std::string word_;
    bool vietnameseEnabled_;
};

} // namespace directtype

#endif // DIRECTTYPE_TYPING_H

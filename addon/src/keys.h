#ifndef DIRECTTYPE_KEYS_H
#define DIRECTTYPE_KEYS_H

#include <cstdint>

// `KeyStates` lives in key.h; `KeyState` and the keysyms come with it.
#include <fcitx-utils/key.h>

namespace directtype {

/// What the addon does with a key event, decided before the engine is involved.
enum class KeyAction {
    /// Ends the current word and is then forwarded to the application: space, tab,
    /// enter, punctuation, a shortcut, a key that produces no character at all.
    Boundary,
    /// Can be part of a word and goes to the engine as `Input::Char`: letters, digits,
    /// and the Telex keys `[` and `]`.
    Character,
    /// Deletes the last keystroke of the word instead of a character of the application's
    /// text. `classify` never returns it: backspace is special-cased where the word is,
    /// because only the caller knows whether there is a word to delete from.
    Backspace,
};

/// Classify one key event.
///
/// The rule is a product decision, not a Unicode property. The engine needs a boundary
/// for anything that ends a word, and a character for anything that can build one; feeding
/// a space as a character would make it part of the word and lose the transformation at
/// the next boundary. Word characters are therefore ASCII letters and digits, `[` and `]`
/// (Telex), the Latin ranges a Vietnamese layout produces (Latin-1 Supplement, Latin
/// Extended-A and Latin Extended-B: `ă`, `ơ`, `ư`, `đ`), and Latin Extended Additional
/// (`ệ`, `ỹ`). Anything else is a boundary: it ends the word and reaches the application
/// untouched.
KeyAction classify(fcitx::KeySym symbol, fcitx::KeyStates states);

/// Whether `codepoint` belongs to a word, i.e. can be typed inside a syllable.
bool isWordCharacter(uint32_t codepoint);

} // namespace directtype

#endif // DIRECTTYPE_KEYS_H

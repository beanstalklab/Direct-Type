// Checks the decision the addon makes before the engine is involved: which keystrokes are
// word characters (letters, digits, `[`, `]`, and the Vietnamese letters a layout produces)
// and which are boundaries (space, punctuation, shortcuts, keys that type nothing).
//
// Getting this wrong is invisible in a session until it matters — a space fed as a word
// character loses the transformation of the word before it — so it is checked here.

#include <cstdint>
#include <cstdio>

#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>

#include "keys.h"

namespace {

int failures = 0;

void check(bool condition, const char *message) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", message);
        failures += 1;
    }
}

void expectCharacter(uint32_t codepoint, const char *name) {
    const fcitx::KeySym symbol = fcitx::Key::keySymFromUnicode(codepoint);
    check(directtype::isWordCharacter(codepoint), name);
    check(directtype::classify(symbol, fcitx::KeyState::NoState) ==
              directtype::KeyAction::Character,
          name);
}

void expectBoundary(uint32_t codepoint, const char *name) {
    const fcitx::KeySym symbol = fcitx::Key::keySymFromUnicode(codepoint);
    check(!directtype::isWordCharacter(codepoint), name);
    check(directtype::classify(symbol, fcitx::KeyState::NoState) ==
              directtype::KeyAction::Boundary,
          name);
}

} // namespace

int main() {
    // Letters, digits and Telex's two letter keys build a word.
    expectCharacter('a', "a is a word character");
    expectCharacter('Z', "Z is a word character");
    expectCharacter('5', "5 is a word character (VNI tones are digits)");
    expectCharacter('[', "[ is a word character (Telex or with horn)");
    expectCharacter(']', "] is a word character (Telex u with horn)");

    // The Vietnamese letters a layout produces directly, in both Latin ranges.
    expectCharacter(0x00E2, "a with circumflex is a word character");
    expectCharacter(0x0103, "a with breve is a word character");
    expectCharacter(0x01A1, "o with horn is a word character");
    expectCharacter(0x01B0, "u with horn is a word character");
    expectCharacter(0x0111, "d with stroke is a word character");
    expectCharacter(0x1EC7, "e with circumflex and dot below is a word character");

    // Anything that ends a word is a boundary and reaches the application untouched.
    expectBoundary(' ', "space ends a word");
    expectBoundary('\t', "tab ends a word");
    expectBoundary(',', "a comma ends a word");
    expectBoundary('.', "a full stop ends a word");
    expectBoundary('"', "a quote ends a word");
    expectBoundary(0x1F600, "an emoji ends a word");

    // A shortcut is not text: Ctrl+C must end the word and go to the application.
    const fcitx::KeySym c = fcitx::Key::keySymFromUnicode('c');
    check(directtype::classify(c, fcitx::KeyState::Ctrl) == directtype::KeyAction::Boundary,
          "Ctrl+C is a boundary, not the letter c");
    check(directtype::classify(c, fcitx::KeyState::Alt) == directtype::KeyAction::Boundary,
          "Alt+C is a boundary");
    const fcitx::KeySym uppercaseC = fcitx::Key::keySymFromUnicode('C');
    // Shift is not a shortcut modifier: it is how a capital is typed, and the frontend has
    // already applied it to the keysym by the time the engine sees the event.
    check(directtype::classify(uppercaseC, fcitx::KeyState::Shift) ==
              directtype::KeyAction::Character,
          "Shift+C typed as C is a word character");

    // Keys that type nothing at all.
    check(directtype::classify(FcitxKey_F1, fcitx::KeyState::NoState) ==
              directtype::KeyAction::Boundary,
          "F1 is a boundary");
    check(directtype::classify(FcitxKey_Left, fcitx::KeyState::NoState) ==
              directtype::KeyAction::Boundary,
          "the left arrow is a boundary");

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("key mapping check passed\n");
    return 0;
}

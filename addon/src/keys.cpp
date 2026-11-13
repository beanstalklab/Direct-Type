#include "keys.h"

#include <fcitx-utils/key.h>

namespace directtype {
namespace {

/// Modifiers that mean "this is a shortcut, not text". Shift is not one of them: shift is
/// how a capital is typed, and CapsLock is a keyboard state rather than a key press.
bool isShortcutModifier(fcitx::KeyStates states) {
    return states.testAny(fcitx::KeyState::Ctrl) || states.testAny(fcitx::KeyState::Alt) ||
           states.testAny(fcitx::KeyState::Super);
}

} // namespace

bool isWordCharacter(uint32_t codepoint) {
    if (codepoint >= 'a' && codepoint <= 'z') {
        return true;
    }
    if (codepoint >= 'A' && codepoint <= 'Z') {
        return true;
    }
    if (codepoint >= '0' && codepoint <= '9') {
        return true;
    }
    // Telex's two letter-producing keys; VNI uses digits, already above.
    if (codepoint == '[' || codepoint == ']') {
        return true;
    }
    // Latin-1 Supplement, Latin Extended-A and Latin Extended-B, which is where every
    // letter a Vietnamese layout produces natively lives: `â` (0x00E2) and `đ` (0x0111) in
    // the first two, `ơ` (0x01A1) and `ư` (0x01B0) in Latin Extended-B.
    if (codepoint >= 0x00C0 && codepoint <= 0x024F) {
        return true;
    }
    // Latin Extended Additional: the precomposed Vietnamese letters with tone marks,
    // `ệ`, `ỹ`, `ử`.
    if (codepoint >= 0x1E00 && codepoint <= 0x1EFF) {
        return true;
    }
    return false;
}

KeyAction classify(fcitx::KeySym symbol, fcitx::KeyStates states) {
    if (isShortcutModifier(states)) {
        return KeyAction::Boundary;
    }
    const uint32_t codepoint = fcitx::Key::keySymToUnicode(symbol);
    if (codepoint == 0 || !isWordCharacter(codepoint)) {
        return KeyAction::Boundary;
    }
    return KeyAction::Character;
}

} // namespace directtype

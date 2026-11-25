#include "rewrite.h"

namespace directtype {

fcitx::DirectTypeRewrite RewriteChoice::decide(fcitx::DirectTypeRewrite configured,
                                              DeliveryFacts facts) {
    if (configured != fcitx::DirectTypeRewrite::Auto) {
        // A strategy the user picked is not a guess about the application, so it applies
        // immediately rather than waiting for the next word.
        current_ = configured;
        decided_ = true;
        return current_;
    }

    if (!decided_) {
        // The one guess in the addon, in falling order of quality: rewriting committed text
        // is invisible when the application cooperates, forwarding keys is invisible when the
        // frontend keeps them in order, and a preedit always shows the word — underlined.
        if (facts.surroundingTextUsable) {
            current_ = fcitx::DirectTypeRewrite::SurroundingText;
        } else if (facts.forwardedKeysOrdered) {
            current_ = fcitx::DirectTypeRewrite::Backspace;
        } else {
            current_ = fcitx::DirectTypeRewrite::Preedit;
        }
        decided_ = true;
    }
    return current_;
}

void RewriteChoice::endWord() { decided_ = false; }

} // namespace directtype

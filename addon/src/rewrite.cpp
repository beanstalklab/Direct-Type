#include "rewrite.h"

namespace directtype {

fcitx::DirectTypeRewrite RewriteChoice::decide(fcitx::DirectTypeRewrite configured,
                                              bool surroundingTextUsable) {
    if (configured != fcitx::DirectTypeRewrite::Auto) {
        // A strategy the user picked is not a guess about the application, so it applies
        // immediately rather than waiting for the next word.
        current_ = configured;
        decided_ = true;
        return current_;
    }

    if (!decided_) {
        // The one guess in the addon: an application that sends surrounding text can be
        // rewritten in place, one that does not cannot (fcitx5's Wayland frontend drops the
        // deletion silently, so the mistake is only visible as broken text).
        current_ = surroundingTextUsable ? fcitx::DirectTypeRewrite::SurroundingText
                                         : fcitx::DirectTypeRewrite::Preedit;
        decided_ = true;
    }
    return current_;
}

void RewriteChoice::endWord() { decided_ = false; }

} // namespace directtype

#ifndef DIRECTTYPE_REWRITE_H
#define DIRECTTYPE_REWRITE_H

#include "config.h"

namespace directtype {

/// What the addon can learn about the application in front of it.
struct DeliveryFacts {
    /// The application sends the text around the cursor, so a deletion can be mapped onto it.
    /// Both halves matter, and the frontend only reports the first.
    bool surroundingTextUsable = false;
    /// The frontend delivers forwarded keys in order with the committed text. fcitx5's X11
    /// (XIM) path does: the key goes through the same X connection as everything else. Its
    /// Wayland path does not: a forwarded key goes through a virtual keyboard, a different
    /// protocol from the input-method commit, so the two can arrive in either order.
    bool forwardedKeysOrdered = false;
};

/// Chooses how the word in progress reaches the application, once per word.
///
/// The choice has to survive the keystrokes of the word it was made for. Asking again on
/// every keystroke would let an application whose surrounding text becomes valid (or stops
/// being valid) change mechanism halfway through a word, and both directions corrupt text:
/// switching to the preedit path shows the whole word on top of the part the application
/// already has, and switching to the direct path deletes characters the application never
/// received. So the answer is latched when the word starts and released when it ends.
///
/// This is policy without I/O, which is why it lives apart from the fcitx5 adapter and can be
/// checked without a session.
class RewriteChoice {
public:
    /// The strategy for the word being typed. An explicit setting always wins and is never
    /// cached against the application, so changing it takes effect at the next keystroke;
    /// `Auto` is resolved from `facts` the first time it is asked in a word.
    fcitx::DirectTypeRewrite decide(fcitx::DirectTypeRewrite configured,
                                    DeliveryFacts facts);

    /// The word ended: the next one decides again.
    void endWord();

    /// Whether this word's strategy has been chosen yet, for a caller that wants to report it.
    bool decided() const { return decided_; }

private:
    fcitx::DirectTypeRewrite current_ = fcitx::DirectTypeRewrite::Preedit;
    bool decided_ = false;
};

} // namespace directtype

#endif // DIRECTTYPE_REWRITE_H

#ifndef DIRECTTYPE_TEXT_TARGET_H
#define DIRECTTYPE_TEXT_TARGET_H

#include <string>

namespace directtype {

/// What the typing logic needs from the application: delete characters before the caret and
/// insert text. fcitx5 implements this with `InputContext::deleteSurroundingText` and
/// `commitString`; a test implements it on a `std::string`.
///
/// Two methods, on purpose: the typing logic then depends on this seam instead of on
/// fcitx5, so it can be checked without a session, and the fcitx5 details (surrounding text
/// capability, backspace fallback) stay in one adapter.
class TextTarget {
public:
    virtual ~TextTarget() = default;

    /// Delete `count` characters, not bytes, before the caret.
    virtual void deleteBeforeCaret(unsigned int count) = 0;

    /// Insert `text` at the caret.
    virtual void insert(const std::string &text) = 0;
};

} // namespace directtype

#endif // DIRECTTYPE_TEXT_TARGET_H

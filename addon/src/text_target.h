#ifndef DIRECTTYPE_TEXT_TARGET_H
#define DIRECTTYPE_TEXT_TARGET_H

#include <string>

namespace directtype {

/// What the typing logic needs from the application: show the word that is being typed, and
/// commit it when it is finished.
///
/// Two methods, on purpose: the typing logic depends on this seam instead of on fcitx5, so it
/// can be checked without a session, and the fcitx5 details (client preedit versus panel
/// preedit) stay in one adapter.
///
/// The word lives in the preedit until a boundary, because Linux applications cannot be
/// trusted to let an input method rewrite text they already received: Sublime Text and some
/// terminals accept the commit and drop the deletion, which turns `việt` into `vĐiệỗt`-shaped
/// garbage. UniKey and Bamboo hold a preedit for the same reason.
class TextTarget {
public:
    virtual ~TextTarget() = default;

    /// Show `text` as the word in progress. An empty string clears the preedit.
    virtual void showPreedit(const std::string &text) = 0;

    /// Give the finished word to the application.
    virtual void commit(const std::string &text) = 0;
};

} // namespace directtype

#endif // DIRECTTYPE_TEXT_TARGET_H

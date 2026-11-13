#ifndef DIRECTTYPE_ENGINE_H
#define DIRECTTYPE_ENGINE_H

#include <vector>

#include <fcitx/inputmethodengine.h>

#include "config.h"
#include "keys.h"
#include "typing.h"

/// The addon's engine: one input method, two languages, one Vietnamese state machine.
///
/// The typing path and the word in progress live in `TypingSession`, which knows nothing
/// about fcitx5; this class is the adapter around it: classify the key, apply the toggle
/// key, render the word as a preedit, commit it at the boundary, and persist the settings.
///
/// The word is kept in a preedit because Linux applications cannot be relied on to let an
/// input method rewrite text they already received — Sublime Text and some terminals accept
/// the commit and drop the deletion. UniKey and Bamboo do the same, for the same reason.
///
/// `InputMethodEngineV2` rather than `InputMethodEngine` because the sub-mode label — the
/// "Vietnamese"/"English" text in fcitx5's status area — only exists on V2:
/// `InputMethodEngine::subModeLabel` dynamic-casts to V2 and returns an empty string
/// otherwise, so overriding `subMode()` on the base would show nothing at all.
class DirectTypeEngine final : public fcitx::InputMethodEngineV2 {
public:
    DirectTypeEngine();
    ~DirectTypeEngine() override;

    DirectTypeEngine(const DirectTypeEngine &) = delete;
    DirectTypeEngine &operator=(const DirectTypeEngine &) = delete;

    /// The single input method this addon provides. Vietnamese and English are two states
    /// of it, not two entries in fcitx5's list.
    std::vector<fcitx::InputMethodEntry> listInputMethods() override;

    void keyEvent(const fcitx::InputMethodEntry &entry,
                  fcitx::KeyEvent &keyEvent) override;

    /// A word ends when the input context goes away, so an invalid word is given back
    /// before the state is dropped.
    void reset(const fcitx::InputMethodEntry &entry,
               fcitx::InputContextEvent &event) override;

    /// What the status area shows next to the input method name: the active language.
    std::string subModeLabelImpl(const fcitx::InputMethodEntry &entry,
                                 fcitx::InputContext &inputContext) override;

    /// The language state and the toggle key, persisted under
    /// `~/.config/fcitx5/conf/directtype.conf`.
    const fcitx::Configuration *getConfig() const override;
    void setConfig(const fcitx::RawConfig &rawConfig) override;
    void save() override;
    void reloadConfig() override;

private:
    /// Show the word in the preedit, or commit it when the word is finished.
    void publish(const directtype::TypingResult &result, fcitx::InputContext *inputContext);

    /// Finish the word in the preedit by committing it. Used when the word has to end for a
    /// reason the engine does not see: a toggle, a configuration change, a lost focus.
    void commitWord(fcitx::InputContext *inputContext);

    /// Push the persisted settings into the session.
    void applyConfiguration();

    directtype::DirectTypeConfig config_;
    directtype::TypingSession session_;
};

#endif // DIRECTTYPE_ENGINE_H

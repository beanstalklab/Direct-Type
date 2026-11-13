#ifndef DIRECTTYPE_ENGINE_H
#define DIRECTTYPE_ENGINE_H

#include <vector>

#include <fcitx/inputmethodengine.h>

#include "config.h"
#include "keys.h"
#include "typing.h"

/// The addon's engine: one input method, two languages, one Vietnamese state machine.
///
/// The typing path and the language state live in `TypingSession`, which knows nothing
/// about fcitx5; this class is the adapter around it: classify the key, apply the toggle
/// key, submit the rest, and persist the settings.
class DirectTypeEngine final : public fcitx::InputMethodEngine {
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

    /// The language state and the toggle key, persisted under
    /// `~/.config/fcitx5/conf/directtype.conf`.
    const fcitx::Configuration *getConfig() const override;
    void setConfig(const fcitx::RawConfig &rawConfig) override;
    void save() override;
    void reloadConfig() override;

private:
    /// Feed one classified key and apply the result to `inputContext`.
    bool consume(directtype::KeyAction action, uint32_t codepoint,
                 fcitx::InputContext *inputContext);

    /// Push the persisted settings into the session.
    void applyConfiguration();

    directtype::DirectTypeConfig config_;
    directtype::TypingSession session_;
};

#endif // DIRECTTYPE_ENGINE_H

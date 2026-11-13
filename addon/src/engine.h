#ifndef DIRECTTYPE_ENGINE_H
#define DIRECTTYPE_ENGINE_H

#include <vector>

#include <fcitx/inputmethodengine.h>

/// The addon's engine: one input method, one language, one state machine.
///
/// T4.1 only proves the addon builds, installs and loads: every key is forwarded
/// untouched, so enabling this input method cannot change what the user types. T4.2 is
/// where the keys go to the Rust engine through `viet.h`.
class DirectTypeEngine final : public fcitx::InputMethodEngine {
public:
    DirectTypeEngine() = default;

    /// The single input method this addon provides.
    std::vector<fcitx::InputMethodEntry> listInputMethods() override;

    void keyEvent(const fcitx::InputMethodEntry &entry,
                  fcitx::KeyEvent &keyEvent) override;
};

#endif // DIRECTTYPE_ENGINE_H

#ifndef DIRECTTYPE_ENGINE_H
#define DIRECTTYPE_ENGINE_H

#include <vector>

#include <fcitx/inputmethodengine.h>

#include "keys.h"
#include "viet.h"

/// The addon's engine: one input method, one language, one Vietnamese state machine.
///
/// It owns a `viet_engine_t` created through the C ABI. The typing path itself lives in
/// `typing.cpp`, which knows nothing about fcitx5; this class is the fcitx5 adapter around
/// it: classify the key, submit it, and implement `TextTarget` on the input context.
class DirectTypeEngine final : public fcitx::InputMethodEngine {
public:
    DirectTypeEngine();
    ~DirectTypeEngine() override;

    DirectTypeEngine(const DirectTypeEngine &) = delete;
    DirectTypeEngine &operator=(const DirectTypeEngine &) = delete;

    /// The single input method this addon provides.
    std::vector<fcitx::InputMethodEntry> listInputMethods() override;

    void keyEvent(const fcitx::InputMethodEntry &entry,
                  fcitx::KeyEvent &keyEvent) override;

    /// A word ends when the input context goes away, so an invalid word is given back
    /// before the state is dropped.
    void reset(const fcitx::InputMethodEntry &entry,
               fcitx::InputContextEvent &event) override;

private:
    /// Feed one classified key and apply the result to `inputContext`.
    bool consume(directtype::KeyAction action, uint32_t codepoint,
                 fcitx::InputContext *inputContext);

    viet_engine_t *engine_;
};

#endif // DIRECTTYPE_ENGINE_H

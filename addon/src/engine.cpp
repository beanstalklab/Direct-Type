#include "engine.h"

#include <string>
#include <utility>

#include <fcitx-config/iniparser.h>
#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/keysym.h>
#include <fcitx/inputcontext.h>

#include "text_target.h"
#include "typing.h"

namespace {

/// Where `readAsIni` and `safeSaveAsIni` put the settings: the user's fcitx5 config
/// directory, which is what survives a restart.
constexpr const char *configurationPath = "conf/directtype.conf";

/// Applies the engine's edits to an fcitx5 input context.
class InputContextTarget final : public directtype::TextTarget {
public:
    explicit InputContextTarget(fcitx::InputContext *inputContext)
        : inputContext_(inputContext) {}

    void deleteBeforeCaret(unsigned int count) override {
        if (inputContext_ == nullptr) {
            return;
        }
        // When the application supports surrounding text, delete by character count.
        // Otherwise forward one backspace key per character, which every frontend
        // understands. Either way it is characters, not bytes: `ệ` is three bytes.
        if (inputContext_->capabilityFlags().test(fcitx::CapabilityFlag::SurroundingText)) {
            inputContext_->deleteSurroundingText(-static_cast<int>(count), count);
        } else {
            for (unsigned int index = 0; index < count; ++index) {
                inputContext_->forwardKey(fcitx::Key(FcitxKey_BackSpace), false);
            }
        }
    }

    void insert(const std::string &text) override {
        if (inputContext_ != nullptr) {
            inputContext_->commitString(text);
        }
    }

private:
    fcitx::InputContext *inputContext_;
};

/// The engine configuration the addon starts with: the documented defaults (spec §2.1),
/// spelled out so the addon cannot drift from them. T4.4 exposes these as options.
viet_config_t engineConfiguration() {
    viet_config_t configuration;
    configuration.method = 0;     // telex
    configuration.tone_style = 1; // new
    configuration.auto_restore = 1;
    configuration.spell_check = 0;
    configuration.free_tone_mark = 1;
    return configuration;
}

} // namespace

DirectTypeEngine::DirectTypeEngine() : config_(), session_(engineConfiguration()) {
    // The state of the previous session, if there is one.
    reloadConfig();
}

DirectTypeEngine::~DirectTypeEngine() = default;

std::vector<fcitx::InputMethodEntry> DirectTypeEngine::listInputMethods() {
    // The last argument is the addon's unique name and has to match `[Addon] Library`
    // in directtype.conf, or fcitx5 cannot route the entry back to this engine.
    fcitx::InputMethodEntry entry("directtype", "Direct Type", "vi", "directtype");
    entry.setNativeName("Tiếng Việt").setLabel("vi");

    std::vector<fcitx::InputMethodEntry> entries;
    entries.push_back(std::move(entry));
    return entries;
}

bool DirectTypeEngine::consume(directtype::KeyAction action, uint32_t codepoint,
                               fcitx::InputContext *inputContext) {
    InputContextTarget target(inputContext);
    return session_.submit(action, codepoint, &target);
}

void DirectTypeEngine::keyEvent(const fcitx::InputMethodEntry &entry,
                                fcitx::KeyEvent &keyEvent) {
    (void)entry;
    if (keyEvent.isRelease()) {
        // A release is not a keystroke: forward it and touch nothing.
        return;
    }

    const fcitx::Key key = keyEvent.key();

    // The toggle key is ours in both languages: it is the only way back out of English.
    if (config_.toggleKey.value().check(key)) {
        session_.setVietnameseEnabled(!session_.vietnameseEnabled());
        config_.vietnameseEnabled.setValue(session_.vietnameseEnabled());
        save();
        keyEvent.filterAndAccept();
        return;
    }

    if (!session_.vietnameseEnabled()) {
        // English: every key reaches the application exactly as typed.
        return;
    }

    const directtype::KeyAction action = directtype::classify(key.sym(), key.states());
    if (action == directtype::KeyAction::Boundary) {
        // The word ends here, and `auto_restore` may rewrite it. The key itself is not
        // consumed, so the space, tab or punctuation still reaches the application.
        consume(action, 0, keyEvent.inputContext());
        return;
    }

    const uint32_t codepoint = fcitx::Key::keySymToUnicode(key.sym());
    if (consume(action, codepoint, keyEvent.inputContext())) {
        // The engine committed the text itself; the key must not also be typed.
        keyEvent.filterAndAccept();
    }
}

void DirectTypeEngine::reset(const fcitx::InputMethodEntry &entry,
                             fcitx::InputContextEvent &event) {
    (void)entry;
    if (session_.vietnameseEnabled()) {
        // Focus loss ends the word, so an invalid one is given back before the state is
        // dropped; whether a commit after focus loss is honoured is the frontend's choice.
        consume(directtype::KeyAction::Boundary, 0, event.inputContext());
    }
    session_.reset();
}

const fcitx::Configuration *DirectTypeEngine::getConfig() const { return &config_; }

void DirectTypeEngine::setConfig(const fcitx::RawConfig &rawConfig) {
    config_.load(rawConfig, true);
    applyConfiguration();
}

void DirectTypeEngine::save() {
    fcitx::safeSaveAsIni(config_, fcitx::StandardPath::Type::PkgConfig, configurationPath);
}

void DirectTypeEngine::reloadConfig() {
    fcitx::readAsIni(config_, fcitx::StandardPath::Type::PkgConfig, configurationPath);
    applyConfiguration();
}

void DirectTypeEngine::applyConfiguration() {
    session_.setVietnameseEnabled(config_.vietnameseEnabled.value());
}

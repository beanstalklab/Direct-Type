#include "engine.h"

#include <string>
#include <utility>

#include <fcitx-config/iniparser.h>
#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputpanel.h>
#include <fcitx/text.h>
#include <fcitx/userinterface.h>

#include "status.h"
#include "text_target.h"
#include "typing.h"

namespace {

/// Where `readAsIni` and `safeSaveAsIni` put the settings: the user's fcitx5 config
/// directory, which is what survives a restart.
constexpr const char *configurationPath = "conf/directtype.conf";

/// Shows the word in progress to the application.
///
/// The preedit is where the word lives until a boundary. When the application supports a
/// client-side preedit it is drawn in the text field itself (underlined, at the cursor);
/// otherwise fcitx5 draws its own preedit window. Either way the application does not receive
/// the text until the word is finished, which is what makes a modifier reliable.
class InputContextTarget final : public directtype::TextTarget {
public:
    InputContextTarget(fcitx::InputContext *inputContext, bool clientPreedit)
        : inputContext_(inputContext), clientPreedit_(clientPreedit) {}

    void showPreedit(const std::string &text) override {
        if (inputContext_ == nullptr) {
            return;
        }
        if (text.empty()) {
            inputContext_->inputPanel().reset();
        } else {
            fcitx::Text preedit(text);
            preedit.setCursor(static_cast<int>(text.size()));
            // Client-side preedit is drawn by the application, at its own cursor. Some
            // applications misplace it while typing fast (Chrome among them), so it is an
            // option: turning it off moves the preedit into fcitx5's own window, which is
            // positioned from the cursor rectangle the frontend reports.
            const bool clientPreedit =
                clientPreedit_ &&
                inputContext_->capabilityFlags().test(fcitx::CapabilityFlag::Preedit);
            if (clientPreedit) {
                inputContext_->inputPanel().setClientPreedit(preedit);
            } else {
                inputContext_->inputPanel().setPreedit(preedit);
            }
        }
        inputContext_->updatePreedit();
    }

    void commit(const std::string &text) override {
        if (inputContext_ != nullptr && !text.empty()) {
            inputContext_->commitString(text);
        }
    }

private:
    fcitx::InputContext *inputContext_;
    bool clientPreedit_;
};

} // namespace

DirectTypeEngine::DirectTypeEngine()
    : config_(), session_(toEngineConfiguration(config_)) {
    // The settings of the previous session, if there are any.
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

void DirectTypeEngine::publish(const directtype::TypingResult &result,
                               fcitx::InputContext *inputContext) {
    InputContextTarget target(inputContext, config_.clientPreedit.value());
    if (result.ended) {
        target.showPreedit("");
        target.commit(result.text);
    } else {
        target.showPreedit(result.text);
    }
}

void DirectTypeEngine::commitWord(fcitx::InputContext *inputContext) {
    if (!session_.hasWord()) {
        return;
    }
    InputContextTarget target(inputContext, config_.clientPreedit.value());
    target.showPreedit("");
    target.commit(session_.takeWord());
}

void DirectTypeEngine::keyEvent(const fcitx::InputMethodEntry &entry,
                                fcitx::KeyEvent &keyEvent) {
    (void)entry;
    if (keyEvent.isRelease()) {
        // A release is not a keystroke: forward it and touch nothing.
        return;
    }

    const fcitx::Key key = keyEvent.key();
    fcitx::InputContext *inputContext = keyEvent.inputContext();

    // The toggle key is ours in both languages: it is the only way back out of English.
    if (config_.toggleKey.value().check(key)) {
        // Finish the word first: a language switch must not leave a half-typed word in a
        // preedit that nothing will ever commit.
        commitWord(inputContext);
        session_.setVietnameseEnabled(!session_.vietnameseEnabled());
        config_.vietnameseEnabled.setValue(session_.vietnameseEnabled());
        save();
        if (inputContext != nullptr) {
            inputContext->updateUserInterface(fcitx::UserInterfaceComponent::StatusArea, true);
        }
        keyEvent.filterAndAccept();
        return;
    }

    if (!session_.vietnameseEnabled()) {
        // English: every key reaches the application exactly as typed.
        return;
    }

    // A backspace inside the word deletes a keystroke from it, not a character from the
    // application: the application has not seen the word yet.
    if (key.sym() == FcitxKey_BackSpace && session_.hasWord()) {
        publish(session_.type(directtype::KeyAction::Backspace, 0), inputContext);
        keyEvent.filterAndAccept();
        return;
    }

    const directtype::KeyAction action = directtype::classify(key.sym(), key.states());
    if (action == directtype::KeyAction::Boundary) {
        // The word ends here: `auto_restore` may rewrite it, and then it is committed. The
        // boundary key itself is not consumed, so the space still reaches the application.
        publish(session_.type(action, 0), inputContext);
        return;
    }

    const uint32_t codepoint = fcitx::Key::keySymToUnicode(key.sym());
    const directtype::TypingResult result = session_.type(action, codepoint);
    if (result.consumed) {
        // The key was typed into the preedit, so the application must not also receive it.
        publish(result, inputContext);
        keyEvent.filterAndAccept();
    }
}

void DirectTypeEngine::reset(const fcitx::InputMethodEntry &entry,
                             fcitx::InputContextEvent &event) {
    (void)entry;
    // Focus loss or an input method switch ends the word: commit it rather than dropping what
    // the user typed. Whether a commit after focus loss is honoured is the frontend's choice.
    commitWord(event.inputContext());
    session_.reset();
}

std::string DirectTypeEngine::subModeLabelImpl(const fcitx::InputMethodEntry &entry,
                                               fcitx::InputContext &inputContext) {
    (void)entry;
    (void)inputContext;
    return directtype::statusLabel(session_.vietnameseEnabled());
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
    session_.setEngineConfiguration(toEngineConfiguration(config_));
    session_.setVietnameseEnabled(config_.vietnameseEnabled.value());
}

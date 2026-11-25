#include "engine.h"

#include <cstdlib>
#include <string>
#include <utility>

#include <fcitx-utils/log.h>

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
/// The preedit is where the word lives until a boundary: the application draws it in the text
/// field itself, underlined, at its own cursor, and receives no text until the word is
/// finished.
///
/// The panel preedit is deliberately not used. fcitx5's own preedit window on Wayland is an
/// `zwp_input_panel_v1` surface, which GNOME does not implement, so that window never appears
/// there and turning the client preedit off would leave the word invisible.
class InputContextTarget final : public directtype::TextTarget {
public:
    explicit InputContextTarget(fcitx::InputContext *inputContext)
        : inputContext_(inputContext) {}

    void showPreedit(const std::string &text) override {
        if (inputContext_ == nullptr) {
            return;
        }
        if (text.empty()) {
            inputContext_->inputPanel().reset();
        } else {
            fcitx::Text preedit(text);
            preedit.setCursor(static_cast<int>(text.size()));
            inputContext_->inputPanel().setClientPreedit(preedit);
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
};

/// Whether the application lets an input method rewrite text it has already received. Both
/// halves matter: the capability says the application implements the protocol, and valid
/// surrounding text says it has actually told us what is there. fcitx5's Wayland frontend
/// silently drops a deletion it cannot map to surrounding text, so a false positive here is
/// visible only as broken text.
bool surroundingTextUsable(fcitx::InputContext *inputContext) {
    return inputContext != nullptr &&
           inputContext->capabilityFlags().test(fcitx::CapabilityFlag::SurroundingText) &&
           inputContext->surroundingText().isValid();
}

/// Whether this frontend delivers a forwarded key in order with the committed text.
///
/// The X11 (XIM) frontend sends the key event on the same X connection as the commit, so a
/// backspace always lands before the text that follows it. The Wayland frontend sends
/// forwarded keys through a virtual keyboard and commits through the input-method protocol:
/// two different paths, with no ordering guarantee, which is how `DĐoôỗ` happens.
bool forwardedKeysOrdered(fcitx::InputContext *inputContext) {
    return inputContext != nullptr && inputContext->frontendName() == "xim";
}

/// The name of a strategy, for a log line.
const char *deliveryName(fcitx::DirectTypeRewrite delivery) {
    switch (delivery) {
    case fcitx::DirectTypeRewrite::Preedit:
        return "Preedit";
    case fcitx::DirectTypeRewrite::Backspace:
        return "Backspace";
    case fcitx::DirectTypeRewrite::SurroundingText:
        return "SurroundingText";
    }
    return "unknown";
}

/// Whether to print what this input context offers. A user diagnosing an application runs
/// fcitx5 with `DIRECTTYPE_DEBUG=1` and reads the lines; nothing is printed otherwise, because
/// one line per word would fill the journal during normal typing.
bool debugEnabled() {
    const char *value = std::getenv("DIRECTTYPE_DEBUG");
    return value != nullptr && value[0] != '\0';
}

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
    if (config_.delivery.value() != fcitx::DirectTypeRewrite::Preedit) {
        applyDirectly(result, inputContext);
    } else {
        InputContextTarget target(inputContext);
        if (result.ended) {
            target.showPreedit("");
            target.commit(result.text);
        } else {
            target.showPreedit(result.text);
        }
    }
}

void DirectTypeEngine::applyDirectly(const directtype::TypingResult &result,
                                     fcitx::InputContext *inputContext) {
    if (inputContext == nullptr) {
        return;
    }

    if (result.backspaces > 0) {
        // Two ways to delete, and neither works everywhere: the surrounding-text API is one
        // atomic request that some applications accept and ignore, and forwarded backspace
        // keys work in any field that accepts keys. That is why the strategy is a setting.
        const bool surroundingText =
            config_.delivery.value() == fcitx::DirectTypeRewrite::SurroundingText &&
            surroundingTextUsable(inputContext);
        if (surroundingText) {
            inputContext->deleteSurroundingText(-static_cast<int>(result.backspaces),
                                                result.backspaces);
        } else {
            for (uint16_t index = 0; index < result.backspaces; ++index) {
                inputContext->forwardKey(fcitx::Key(FcitxKey_BackSpace), false);
            }
        }
    }

    if (!result.commit.empty()) {
        inputContext->commitString(result.commit);
    }
}

void DirectTypeEngine::commitWord(fcitx::InputContext *inputContext) {
    if (!session_.hasWord()) {
        return;
    }

    if (config_.delivery.value() != fcitx::DirectTypeRewrite::Preedit) {
        // The application already has the text: there is no preedit to clear. End the word
        // the way a boundary would, so `auto_restore` still gets its say.
        applyDirectly(session_.type(directtype::KeyAction::Boundary, 0), inputContext);
    } else {
        InputContextTarget target(inputContext);
        target.showPreedit("");
        target.commit(session_.takeWord());
    }
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
        // With no word there is nothing to end, and no preedit to clear.
        const directtype::TypingResult result = session_.type(action, 0);
        if (result.ended) {
            publish(result, inputContext);
        }
        return;
    }

    const uint32_t codepoint = fcitx::Key::keySymToUnicode(key.sym());
    const bool startsWord = !session_.hasWord();
    const directtype::TypingResult result = session_.type(action, codepoint);
    if (result.consumed) {
        // The key was typed into the preedit, so the application must not also receive it.
        publish(result, inputContext);
        keyEvent.filterAndAccept();
    }
    if (startsWord && debugEnabled()) {
        // One line per word, on request: what the application says it can do, next to what
        // the user chose. That is the pair that explains a report like "garbage in the
        // address bar but fine in the page".
        FCITX_INFO() << "directtype: program="
                     << (inputContext != nullptr ? inputContext->program() : std::string())
                     << " frontend="
                     << (inputContext != nullptr
                             ? std::string(inputContext->frontendName())
                             : std::string())
                     << " surroundingText=" << surroundingTextUsable(inputContext)
                     << " orderedForwardKeys=" << forwardedKeysOrdered(inputContext)
                     << " delivery=" << deliveryName(config_.delivery.value());
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
    session_.setEngineConfiguration(directtype::toEngineConfiguration(config_));
    session_.setVietnameseEnabled(config_.vietnameseEnabled.value());
}

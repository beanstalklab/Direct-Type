#include "typing.h"

namespace directtype {
namespace {

viet_input_t characterInput(uint32_t codepoint) {
    viet_input_t input;
    input.kind = 0;
    input.code_point = codepoint;
    return input;
}

viet_input_t simpleInput(int32_t kind) {
    viet_input_t input;
    input.kind = kind; // 1 backspace, 2 boundary, 3 reset
    input.code_point = 0;
    return input;
}

/// Remove one UTF-8 character from the end of `text`: the engine counts deletions in
/// characters, and `ệ` is three bytes.
void popCharacter(std::string &text) {
    while (!text.empty() && (static_cast<unsigned char>(text.back()) & 0xC0) == 0x80) {
        text.pop_back();
    }
    if (!text.empty()) {
        text.pop_back();
    }
}

} // namespace

TypingSession::TypingSession(const viet_config_t &configuration)
    : engine_(viet_engine_new(&configuration)), vietnameseEnabled_(true) {}

TypingSession::~TypingSession() {
    if (engine_ != nullptr) {
        viet_engine_free(engine_);
    }
}

std::string TypingSession::takeWord() {
    std::string word = word_;
    word_.clear();
    return word;
}

void TypingSession::feed(const viet_input_t &input) {
    if (engine_ == nullptr) {
        return;
    }
    viet_action_t action;
    const char *commit = nullptr;
    viet_engine_feed(engine_, &input, &action, &commit, nullptr);

    // The action is a deletion-and-insert against the text the engine has shown so far, which
    // is exactly this buffer.
    for (uint16_t index = 0; index < action.backspaces; ++index) {
        popCharacter(word_);
    }
    if (action.commit_length > 0 && commit != nullptr) {
        word_.append(commit, action.commit_length);
    }
}

TypingResult TypingSession::type(KeyAction action, uint32_t codepoint) {
    TypingResult result;
    if (engine_ == nullptr || !vietnameseEnabled_) {
        return result;
    }

    switch (action) {
    case KeyAction::Backspace:
        if (word_.empty()) {
            // Nothing of ours to delete: the application deletes its own text.
            return result;
        }
        feed(simpleInput(1));
        result.consumed = true;
        result.text = word_;
        return result;
    case KeyAction::Character:
        feed(characterInput(codepoint));
        result.consumed = true;
        result.text = word_;
        return result;
    case KeyAction::Boundary:
        if (word_.empty()) {
            return result;
        }
        // `auto_restore` may rewrite the finished word; that rewrite belongs in the preedit,
        // and then the whole word is committed.
        feed(simpleInput(2));
        result.ended = true;
        result.text = takeWord();
        return result;
    }
    return result;
}

void TypingSession::setVietnameseEnabled(bool enabled) {
    if (enabled == vietnameseEnabled_) {
        return;
    }
    reset();
    vietnameseEnabled_ = enabled;
}

void TypingSession::setEngineConfiguration(const viet_config_t &configuration) {
    if (engine_ != nullptr) {
        viet_engine_set_config(engine_, &configuration);
    }
}

void TypingSession::reset() {
    word_.clear();
    if (engine_ != nullptr) {
        viet_engine_reset(engine_);
    }
}

} // namespace directtype

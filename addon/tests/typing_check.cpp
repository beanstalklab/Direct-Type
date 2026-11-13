// Types words through the addon's own typing path (`typing.cpp`), against a string instead
// of an fcitx5 input context: the key classification, the engine call, the preedit and the
// commit, all together.
//
// What it cannot check is the last mile — that fcitx5 calls `keyEvent`, and that the
// application renders the preedit and accepts the commit — which stays a human check.

#include <cstdint>
#include <cstdio>
#include <string>

#include "keys.h"
#include "text_target.h"
#include "typing.h"
#include "viet.h"

namespace {

int failures = 0;

void check(bool condition, const char *message) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", message);
        failures += 1;
    }
}

/// Compare text, printing both sides: a wrong byte in a UTF-8 literal is invisible otherwise.
void checkEqual(const std::string &actual, const std::string &expected, const char *message) {
    if (actual != expected) {
        std::fprintf(stderr, "FAIL: %s (got \"%s\", expected \"%s\")\n", message,
                     actual.c_str(), expected.c_str());
        failures += 1;
    }
}

/// An application that renders the preedit and appends what is committed to its text.
class StringTarget final : public directtype::TextTarget {
public:
    void showPreedit(const std::string &text) override { preedit_ = text; }

    void commit(const std::string &text) override {
        committed_ += text;
        preedit_.clear();
    }

    const std::string &committed() const { return committed_; }
    const std::string &preedit() const { return preedit_; }

private:
    std::string committed_;
    std::string preedit_;
};

/// UTF-8 for one code point, for the keys the application types itself.
std::string utf8(uint32_t codepoint) {
    std::string text;
    if (codepoint < 0x80) {
        text.push_back(static_cast<char>(codepoint));
    } else if (codepoint < 0x800) {
        text.push_back(static_cast<char>(0xC0 | (codepoint >> 6)));
        text.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
    } else if (codepoint < 0x10000) {
        text.push_back(static_cast<char>(0xE0 | (codepoint >> 12)));
        text.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3F)));
        text.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
    } else {
        text.push_back(static_cast<char>(0xF0 | (codepoint >> 18)));
        text.push_back(static_cast<char>(0x80 | ((codepoint >> 12) & 0x3F)));
        text.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3F)));
        text.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
    }
    return text;
}

/// The addon's typing path, with the key classification and the language toggle it uses.
class Typer {
public:
    explicit Typer(viet_config_t configuration) : session_(configuration) {}

    /// One key, the way `DirectTypeEngine::keyEvent` drives the session.
    void press(uint32_t codepoint) {
        const fcitx::KeySym symbol = fcitx::Key::keySymFromUnicode(codepoint);
        const directtype::KeyAction action =
            directtype::classify(symbol, fcitx::KeyState::NoState);
        apply(session_.type(action, codepoint), codepoint);
    }

    void type(const std::string &keys) {
        for (char key : keys) {
            press(static_cast<unsigned char>(key));
        }
    }

    void backspace() { apply(session_.type(directtype::KeyAction::Backspace, 0), 0); }

    /// End the word the way a space does: the boundary commits, then the space itself is the
    /// application's.
    void space() {
        apply(session_.type(directtype::KeyAction::Boundary, 0), 0);
        target_.commit(" ");
    }

    /// End the word without typing a key: the engine may rewrite the word before it is
    /// committed.
    std::string finish() {
        const directtype::TypingResult result =
            session_.type(directtype::KeyAction::Boundary, 0);
        if (result.ended) {
            target_.showPreedit("");
            target_.commit(result.text);
        }
        return target_.committed();
    }

    /// What the application shows right now: committed text plus the word being typed.
    std::string text() { return target_.committed() + target_.preedit(); }

    const std::string &preedit() const { return target_.preedit(); }

    bool vietnameseEnabled() const { return session_.vietnameseEnabled(); }

    void setVietnameseEnabled(bool enabled) {
        // The engine finishes the word before a language switch, as the addon does.
        if (session_.hasWord()) {
            target_.showPreedit("");
            target_.commit(session_.takeWord());
        }
        session_.setVietnameseEnabled(enabled);
    }

    void setEngineConfiguration(const viet_config_t &configuration) {
        if (session_.hasWord()) {
            target_.showPreedit("");
            target_.commit(session_.takeWord());
        }
        session_.setEngineConfiguration(configuration);
    }

private:
    void apply(const directtype::TypingResult &result, uint32_t codepoint) {
        if (result.ended) {
            target_.showPreedit("");
            target_.commit(result.text);
        } else if (result.consumed) {
            target_.showPreedit(result.text);
        } else {
            // The key is not ours: the application types it itself.
            target_.commit(utf8(codepoint));
        }
    }

    directtype::TypingSession session_;
    StringTarget target_;
};

viet_config_t telexConfiguration() {
    viet_config_t configuration;
    configuration.method = 0;
    configuration.tone_style = 1;
    configuration.auto_restore = 1;
    configuration.spell_check = 0;
    configuration.free_tone_mark = 1;
    return configuration;
}

void expectWord(const std::string &keys, const std::string &expected, const char *name) {
    Typer typer(telexConfiguration());
    typer.type(keys);
    checkEqual(typer.finish(), expected, name);
}

} // namespace

int main() {
    expectWord("vieejt", "vi\xE1\xBB\x87" "t", "vieejt types viet");
    expectWord("dduowngf", "\xC4\x91\xC6\xB0\xE1\xBB\x9D" "ng", "dduowngf types duong");
    expectWord("uowng", "\xC6\xB0\xC6\xA1" "ng", "uowng horns both letters");
    expectWord("thuowr", "thu\xE1\xBB\x9F", "thuowr keeps the uo rhyme");
    expectWord("asd", "asd", "auto_restore gives the keystrokes back");
    expectWord("aass", "aass", "an undone tone that leaves no syllable is restored");
    expectWord("toans", "to\xC3\xA1n", "the mark moves when the coda arrives");

    // The word lives in the preedit and reaches the application only at the boundary.
    {
        Typer typer(telexConfiguration());
        typer.type("vieejt");
        check(typer.preedit() == "vi\xE1\xBB\x87" "t", "the word is the preedit");
        check(typer.text() == "vi\xE1\xBB\x87" "t", "nothing was committed yet");
        typer.space();
        checkEqual(typer.text(), "vi\xE1\xBB\x87" "t ", "the boundary commits the word and the space");
    }

    // Two words in a row keep their own text.
    {
        Typer typer(telexConfiguration());
        typer.type("vieejt");
        typer.space();
        typer.type("asd");
        typer.space();
        typer.type("aas");
        check(typer.finish() == "vi\xE1\xBB\x87" "t asd \xE1\xBA\xA5",
              "two words and a space keep their own text");
    }

    // Backspace in the word deletes a keystroke, not a character of the application.
    {
        Typer typer(telexConfiguration());
        typer.type("aas");
        check(typer.preedit() == "\xE1\xBA\xA5", "aas is a with circumflex and acute");
        typer.backspace();
        check(typer.preedit() == "\xC3\xA2", "backspace deletes the tone key, not the letter");
        check(typer.finish() == "\xC3\xA2", "the word is committed after the delete");
    }

    // A capital is a word character: `Vieejt` is `Việt`.
    {
        Typer typer(telexConfiguration());
        typer.press('V');
        typer.type("ieejt");
        check(typer.finish() == "Vi\xE1\xBB\x87" "t", "a capital stays a capital");
    }

    // VNI through the same path.
    {
        viet_config_t configuration = telexConfiguration();
        configuration.method = 2;
        Typer typer(configuration);
        typer.type("d9u7o7ng2");
        check(typer.finish() == "\xC4\x91\xC6\xB0\xE1\xBB\x9D" "ng",
              "VNI types duong through the same path");
    }

    // English is the same input method with the engine switched off: every key reaches the
    // application exactly as typed.
    {
        Typer typer(telexConfiguration());
        check(typer.vietnameseEnabled(), "Vietnamese is on at the start");
        typer.setVietnameseEnabled(false);
        typer.type("vieejt");
        checkEqual(typer.finish(), "vieejt", "English forwards every key untouched");
    }

    // A toggle finishes the word instead of dropping it, and the next word starts fresh.
    {
        Typer typer(telexConfiguration());
        typer.type("asd");
        check(typer.preedit() == "\xC3\xA1" "d", "a live word is in the preedit");
        typer.setVietnameseEnabled(false);
        checkEqual(typer.text(), "\xC3\xA1" "d", "turning Vietnamese off commits the word, editing nothing");
        typer.setVietnameseEnabled(true);
        typer.type("a");
        checkEqual(typer.text(), "\xC3\xA1" "d" "a",
                   "the next word starts fresh, without touching the previous one");
        typer.type("s");
        checkEqual(typer.text(), "\xC3\xA1" "d\xC3\xA1", "the fresh word transforms on its own");
    }

    // A settings change finishes the word too: the engine would otherwise drop text the user
    // already sees in the preedit.
    {
        Typer typer(telexConfiguration());
        typer.type("viee");
        check(typer.preedit() == "vi\xC3\xAA", "a word is in progress");
        viet_config_t vni = telexConfiguration();
        vni.method = 2;
        typer.setEngineConfiguration(vni);
        checkEqual(typer.text(), "vi\xC3\xAA", "changing the method commits the word it had");
        typer.type("d9");
        check(typer.preedit() == "\xC4\x91", "the next word starts fresh, in the new method");
    }

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("typing check passed\n");
    return 0;
}

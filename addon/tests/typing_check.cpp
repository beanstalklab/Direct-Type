// Types words through the addon's own typing path (`typing.cpp`), against a string instead
// of an fcitx5 input context. This is the headless half of T4.2: the key classification, the
// engine call, the character-wise deletion and the commit, all together.
//
// What it cannot check is the last mile — that fcitx5 calls `keyEvent` and that the
// application accepts the edits — which stays a human check in a real session.

#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

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

/// A text buffer that behaves like an application: it deletes characters, not bytes, which
/// is the lesson `crates/viet-ffi/tests/smoke.c` recorded for the C ABI.
class StringTarget final : public directtype::TextTarget {
public:
    void deleteBeforeCaret(unsigned int count) override {
        while (count > 0 && !text_.empty()) {
            std::size_t index = text_.size() - 1;
            while (index > 0 && (static_cast<unsigned char>(text_[index]) & 0xC0) == 0x80) {
                index -= 1;
            }
            text_.erase(index);
            count -= 1;
        }
    }

    void insert(const std::string &text) override { text_ += text; }

    const std::string &text() const { return text_; }

private:
    std::string text_;
};

/// The addon's typing path, with the key classification it uses.
class Typer {
public:
    explicit Typer(viet_config_t configuration) {
        engine_ = viet_engine_new(&configuration);
        check(engine_ != nullptr, "the engine is created");
    }

    ~Typer() {
        if (engine_ != nullptr) {
            viet_engine_free(engine_);
        }
    }

    Typer(const Typer &) = delete;
    Typer &operator=(const Typer &) = delete;

    void press(uint32_t codepoint) {
        const fcitx::KeySym symbol = fcitx::Key::keySymFromUnicode(codepoint);
        const directtype::KeyAction action =
            directtype::classify(symbol, fcitx::KeyState::NoState);
        directtype::submit(engine_, action, codepoint, &target_);
    }

    void type(const std::string &keys) {
        for (char key : keys) {
            press(static_cast<unsigned char>(key));
        }
    }

    /// End the word the way a space does: the boundary runs, then the space itself is the
    /// application's.
    void space() {
        directtype::submit(engine_, directtype::KeyAction::Boundary, 0, &target_);
        target_.insert(" ");
    }

    std::string finish() {
        directtype::submit(engine_, directtype::KeyAction::Boundary, 0, &target_);
        return target_.text();
    }

private:
    viet_engine_t *engine_;
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
    check(typer.finish() == expected, name);
}

} // namespace

int main() {
    expectWord("vieejt", "việt", "vieejt types viet");
    expectWord("dduowngf", "\xC4\x91\xC6\xB0\xE1\xBB\x9D" "ng", "dduowngf types duong");
    expectWord("uowng", "\xC6\xB0\xC6\xA1" "ng", "uowng horns both letters");
    expectWord("thuowr", "thu\xE1\xBB\x9F", "thuowr keeps the uo rhyme");
    expectWord("asd", "asd", "auto_restore gives the keystrokes back");
    expectWord("aass", "aass", "an undone tone that leaves no syllable is restored");
    expectWord("toans", "to\xC3\xA1n", "the mark moves when the coda arrives");

    // A word ends at a space and the next one starts fresh, which is what makes
    // `Type::space` and `Input::Boundary` worth two separate calls.
    {
        Typer typer(telexConfiguration());
        typer.type("vieejt");
        typer.space();
        typer.type("asd");
        typer.space();
        typer.type("aas");
        const std::string expected = "vi\xE1\xBB\x87t" " asd \xE1\xBA\xA5";
        check(typer.finish() == expected, "two words and a space keep their own text");
    }

    // A capital is a word character: `Vieejt` is `Việt`.
    {
        Typer typer(telexConfiguration());
        typer.press('V');
        typer.type("ieejt");
        check(typer.finish() == "Vi\xE1\xBB\x87t", "a capital stays a capital");
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

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("typing check passed\n");
    return 0;
}

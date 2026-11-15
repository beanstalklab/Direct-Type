// Checks the one guess the addon makes about an application: how the word reaches it.
//
// The strategy must be chosen once per word, not once per keystroke. A decision that flips
// while a word is being typed corrupts text in both directions — a late preedit shows the
// whole word over the part the application already has, a late direct write deletes
// characters the application never received — and the failure is silent until the user reads
// the result. So the latch is checked here rather than only in a session.

#include <cstdio>

#include "rewrite.h"

namespace {

int failures = 0;

void check(bool condition, const char *message) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", message);
        failures += 1;
    }
}

void expectStrategy(directtype::RewriteChoice &choice, fcitx::DirectTypeRewrite configured,
                    bool surroundingTextUsable, fcitx::DirectTypeRewrite expected,
                    const char *message) {
    check(choice.decide(configured, surroundingTextUsable) == expected, message);
}

} // namespace

int main() {
    // `Auto` asks the application at the start of a word and keeps that answer.
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, true,
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "auto rewrites in place when the application sends surrounding text");
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, false,
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "a word keeps the strategy it started with");
        choice.endWord();
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, false,
                       fcitx::DirectTypeRewrite::Preedit,
                       "the next word decides again");
    }

    // And the other direction: a preedit word does not become a direct one halfway.
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, false,
                       fcitx::DirectTypeRewrite::Preedit,
                       "auto keeps a preedit without surrounding text");
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, true,
                       fcitx::DirectTypeRewrite::Preedit,
                       "a preedit word stays a preedit word");
        choice.endWord();
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, true,
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "the word after it may be rewritable again");
    }

    // An explicit setting is never overridden by what the application can do.
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Preedit, true,
                       fcitx::DirectTypeRewrite::Preedit,
                       "an explicit preedit ignores surrounding text");
        expectStrategy(choice, fcitx::DirectTypeRewrite::Preedit, false,
                       fcitx::DirectTypeRewrite::Preedit,
                       "an explicit preedit holds for the whole word");
        expectStrategy(choice, fcitx::DirectTypeRewrite::Backspace, true,
                       fcitx::DirectTypeRewrite::Backspace,
                       "an explicit backspace rewrite applies immediately");
        expectStrategy(choice, fcitx::DirectTypeRewrite::SurroundingText, false,
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "an explicit surrounding-text rewrite is not second-guessed");
    }

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("rewrite check passed\n");
    return 0;
}

// Checks the one guess the addon makes about an application: how the word reaches it.
//
// Two rules are checked here because both are silent when wrong. The strategy must be chosen
// once per word, not once per keystroke: a decision that flips while a word is being typed
// corrupts text in both directions — a late preedit shows the whole word over the part the
// application already has, a late direct write deletes characters it never received. And the
// fallback order matters: rewriting committed text beats forwarding keys, which beats a
// preedit, and only the frontend can say which of the first two is safe.

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

directtype::DeliveryFacts facts(bool surroundingTextUsable, bool forwardedKeysOrdered) {
    directtype::DeliveryFacts result;
    result.surroundingTextUsable = surroundingTextUsable;
    result.forwardedKeysOrdered = forwardedKeysOrdered;
    return result;
}

void expectStrategy(directtype::RewriteChoice &choice, fcitx::DirectTypeRewrite configured,
                    directtype::DeliveryFacts known, fcitx::DirectTypeRewrite expected,
                    const char *message) {
    check(choice.decide(configured, known) == expected, message);
}

} // namespace

int main() {
    // `Auto` picks the best mechanism the application offers.
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(true, false),
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "auto rewrites in place when the application sends surrounding text");
    }
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(false, true),
                       fcitx::DirectTypeRewrite::Backspace,
                       "auto forwards keys when the frontend keeps them in order");
    }
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(false, false),
                       fcitx::DirectTypeRewrite::Preedit,
                       "auto keeps a preedit when neither rewrite is safe");
    }

    // Surrounding text wins over forwarding keys: it is one atomic request, not a burst of
    // synthetic key events.
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(true, true),
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "rewriting in place beats forwarding keys when both are possible");
    }

    // A word keeps the strategy it started with, even when the application's state changes
    // under it.
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(true, false),
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "the word starts by rewriting in place");
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(false, false),
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "a word keeps the strategy it started with");
        choice.endWord();
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(false, false),
                       fcitx::DirectTypeRewrite::Preedit,
                       "the next word decides again");
    }
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(false, false),
                       fcitx::DirectTypeRewrite::Preedit,
                       "auto keeps a preedit without a safe rewrite");
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(true, true),
                       fcitx::DirectTypeRewrite::Preedit,
                       "a preedit word stays a preedit word");
        choice.endWord();
        expectStrategy(choice, fcitx::DirectTypeRewrite::Auto, facts(true, true),
                       fcitx::DirectTypeRewrite::SurroundingText,
                       "the word after it may be rewritable again");
    }

    // An explicit setting is never overridden by what the application can do.
    {
        directtype::RewriteChoice choice;
        expectStrategy(choice, fcitx::DirectTypeRewrite::Preedit, facts(true, true),
                       fcitx::DirectTypeRewrite::Preedit,
                       "an explicit preedit ignores what the application offers");
        expectStrategy(choice, fcitx::DirectTypeRewrite::Backspace, facts(true, false),
                       fcitx::DirectTypeRewrite::Backspace,
                       "an explicit backspace rewrite applies immediately");
        expectStrategy(choice, fcitx::DirectTypeRewrite::SurroundingText, facts(false, false),
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

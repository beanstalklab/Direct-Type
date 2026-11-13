#include "engine.h"

#include <utility>

std::vector<fcitx::InputMethodEntry> DirectTypeEngine::listInputMethods() {
    // The last argument is the addon's unique name and has to match `[Addon] Library`
    // in directtype.conf, or fcitx5 cannot route the entry back to this engine.
    fcitx::InputMethodEntry entry("directtype", "Direct Type", "vi", "directtype");
    entry.setNativeName("Tiếng Việt").setLabel("vi");

    std::vector<fcitx::InputMethodEntry> entries;
    entries.push_back(std::move(entry));
    return entries;
}

void DirectTypeEngine::keyEvent(const fcitx::InputMethodEntry &entry,
                                fcitx::KeyEvent &keyEvent) {
    // Forward everything. Leaving the event unaccepted is what tells fcitx5 to keep
    // processing it, which is the only sane default while the engine is not wired in.
    (void)entry;
    (void)keyEvent;
}

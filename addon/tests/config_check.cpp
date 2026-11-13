// Checks that the addon's settings survive a restart, and where they live.
//
// The engine persists through fcitx5's own ini machinery into
// `$XDG_CONFIG_HOME/fcitx5/conf/directtype.conf`. The test points XDG_CONFIG_HOME at a
// temporary directory first, so it never touches the settings of the person running it.

#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <string>

#include <fcitx-config/rawconfig.h>
#include <fcitx-utils/key.h>

#include "config.h"
#include "engine.h"

namespace {

int failures = 0;

void check(bool condition, const char *message) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", message);
        failures += 1;
    }
}

const directtype::DirectTypeConfig &configurationOf(DirectTypeEngine &engine) {
    return *static_cast<const directtype::DirectTypeConfig *>(engine.getConfig());
}

} // namespace

int main() {
    const std::filesystem::path temporary =
        std::filesystem::temp_directory_path() / "directtype-config-check";
    std::filesystem::remove_all(temporary);
    std::filesystem::create_directories(temporary);
    setenv("XDG_CONFIG_HOME", temporary.c_str(), 1);

    {
        DirectTypeEngine engine;
        const directtype::DirectTypeConfig &configuration = configurationOf(engine);
        check(configuration.vietnameseEnabled.value(), "Vietnamese is on by default");
        check(configuration.toggleKey.value() == fcitx::Key("Alt+Z"),
              "the toggle key defaults to Alt+Z");

        // Turn English on the way the config UI will: a RawConfig holding only that key.
        fcitx::RawConfig raw;
        raw.setValueByPath("VietnameseEnabled", "False");
        engine.setConfig(raw);
        check(!configurationOf(engine).vietnameseEnabled.value(),
              "a RawConfig turns Vietnamese off");
        engine.save();
    }

    // The file is where a restart will look for it.
    const std::filesystem::path expected = temporary / "fcitx5" / "conf" / "directtype.conf";
    check(std::filesystem::exists(expected),
          "the settings are written under $XDG_CONFIG_HOME/fcitx5/conf");

    // A new engine is a restart: it must come back in English.
    {
        DirectTypeEngine restarted;
        check(!configurationOf(restarted).vietnameseEnabled.value(),
              "the language survives a restart");

        // And it can be turned back on the same way.
        fcitx::RawConfig raw;
        raw.setValueByPath("VietnameseEnabled", "True");
        restarted.setConfig(raw);
        check(configurationOf(restarted).vietnameseEnabled.value(),
              "the language can be turned back on");
    }

    std::filesystem::remove_all(temporary);

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("config check passed\n");
    return 0;
}

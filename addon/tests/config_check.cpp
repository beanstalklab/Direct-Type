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

/// Whether a description names `expected` at `path`.
bool describesAs(const fcitx::RawConfig &description, const std::string &path,
                 const std::string &expected) {
    const std::string *value = description.valueByPath(path);
    return value != nullptr && *value == expected;
}

const directtype::DirectTypeConfig &configurationOf(DirectTypeEngine &engine) {
    return *static_cast<const directtype::DirectTypeConfig *>(engine.getConfig());
}

} // namespace

int main() {
    const std::filesystem::path temporary =
        std::filesystem::temp_directory_path() / "directtype-config-check";
    // What fcitx5-configtool renders: the description has to name every enum value, or the
    // option appears as a text field the user cannot fill in correctly.
    {
        directtype::DirectTypeConfig config;
        fcitx::RawConfig description;
        config.dumpDescription(description);
        check(describesAs(description, "DirectTypeConfig/Method/Enum/0", "Telex"),
              "the method combo offers Telex");
        check(describesAs(description, "DirectTypeConfig/Method/Enum/1", "SimpleTelex"),
              "the method combo offers SimpleTelex");
        check(describesAs(description, "DirectTypeConfig/Method/Enum/2", "Vni"),
              "the method combo offers Vni");
        check(describesAs(description, "DirectTypeConfig/ToneStyle/Enum/0", "Old"),
              "the tone placement combo offers Old");
        check(describesAs(description, "DirectTypeConfig/ToneStyle/Enum/1", "New"),
              "the tone placement combo offers New");
        check(describesAs(description, "DirectTypeConfig/ToggleKey/Type", "Key"),
              "the toggle key is described as a key");
    }

    // Every method maps to its own ABI number, and the enums are ordered as the ABI expects.
    {
        directtype::DirectTypeConfig config;
        config.method.setValue(fcitx::DirectTypeMethod::SimpleTelex);
        check(directtype::toEngineConfiguration(config).method == 1,
              "Simple Telex is 1 in the ABI");
        config.method.setValue(fcitx::DirectTypeMethod::Vni);
        check(directtype::toEngineConfiguration(config).method == 2, "VNI is 2 in the ABI");
        config.toneStyle.setValue(fcitx::DirectTypeToneStyle::Old);
        check(directtype::toEngineConfiguration(config).tone_style == 0,
              "the old tone style is 0 in the ABI");
        config.spellCheck.setValue(true);
        config.autoRestore.setValue(false);
        config.freeToneMark.setValue(false);
        const viet_config_t mapped = directtype::toEngineConfiguration(config);
        check(mapped.spell_check == 1 && mapped.auto_restore == 0 && mapped.free_tone_mark == 0,
              "the switches map to their own fields");
    }

    std::filesystem::remove_all(temporary);
    std::filesystem::create_directories(temporary);
    setenv("XDG_CONFIG_HOME", temporary.c_str(), 1);

    {
        DirectTypeEngine engine;
        const directtype::DirectTypeConfig &configuration = configurationOf(engine);
        check(configuration.vietnameseEnabled.value(), "Vietnamese is on by default");
        check(configuration.toggleKey.value() == fcitx::Key("Alt+Z"),
              "the toggle key defaults to Alt+Z");
        // The defaults have to match the engine's own, or a fresh install would not type
        // what the engine documents.
        check(configuration.method.value() == fcitx::DirectTypeMethod::Telex,
              "the method defaults to Telex");
        check(configuration.toneStyle.value() == fcitx::DirectTypeToneStyle::New,
              "the tone placement defaults to the new style");
        check(configuration.autoRestore.value(), "auto_restore defaults to on");
        check(!configuration.spellCheck.value(), "spell_check defaults to off");
        check(configuration.freeToneMark.value(), "free_tone_mark defaults to on");

        // The options reach the ABI as the numbers `viet.h` documents. A wrong number here is
        // "VNI selected, Telex typed", which no other test would notice.
        const viet_config_t engineConfiguration = directtype::toEngineConfiguration(configuration);
        check(engineConfiguration.method == 0, "the default options are Telex in the ABI");
        check(engineConfiguration.tone_style == 1, "the new tone style is 1 in the ABI");
        check(engineConfiguration.auto_restore == 1, "auto_restore is on in the ABI");
        check(engineConfiguration.spell_check == 0, "spell_check is off in the ABI");
        check(engineConfiguration.free_tone_mark == 1, "free_tone_mark is on in the ABI");

        // Turn English on the way the config UI will: a RawConfig holding only that key.
        fcitx::RawConfig raw;
        raw.setValueByPath("VietnameseEnabled", "False");
        raw.setValueByPath("Method", "Vni");
        raw.setValueByPath("ToneStyle", "Old");
        raw.setValueByPath("SpellCheck", "True");
        engine.setConfig(raw);
        check(!configurationOf(engine).vietnameseEnabled.value(),
              "a RawConfig turns Vietnamese off");
        check(configurationOf(engine).method.value() == fcitx::DirectTypeMethod::Vni,
              "a RawConfig changes the method");
        engine.save();
    }

    // The file is where a restart will look for it.
    const std::filesystem::path expected = temporary / "fcitx5" / "conf" / "directtype.conf";
    check(std::filesystem::exists(expected),
          "the settings are written under $XDG_CONFIG_HOME/fcitx5/conf");

    // A new engine is a restart: it must come back in English.
    {
        DirectTypeEngine restarted;
        const directtype::DirectTypeConfig &reloaded = configurationOf(restarted);
        check(!reloaded.vietnameseEnabled.value(), "the language survives a restart");
        check(reloaded.method.value() == fcitx::DirectTypeMethod::Vni,
              "the method survives a restart");
        check(reloaded.toneStyle.value() == fcitx::DirectTypeToneStyle::Old,
              "the tone placement survives a restart");
        check(reloaded.spellCheck.value(), "spell_check survives a restart");
        check(reloaded.autoRestore.value(), "an untouched option keeps its default");

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

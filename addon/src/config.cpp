#include "config.h"

namespace directtype {

viet_config_t toEngineConfiguration(const DirectTypeConfig &config) {
    viet_config_t configuration;
    switch (config.method.value()) {
    case fcitx::DirectTypeMethod::SimpleTelex:
        configuration.method = 1;
        break;
    case fcitx::DirectTypeMethod::Vni:
        configuration.method = 2;
        break;
    case fcitx::DirectTypeMethod::Telex:
    default:
        configuration.method = 0;
        break;
    }
    configuration.tone_style =
        config.toneStyle.value() == fcitx::DirectTypeToneStyle::New ? 1 : 0;
    configuration.auto_restore = config.autoRestore.value() ? 1 : 0;
    configuration.spell_check = config.spellCheck.value() ? 1 : 0;
    configuration.free_tone_mark = config.freeToneMark.value() ? 1 : 0;
    return configuration;
}

} // namespace directtype

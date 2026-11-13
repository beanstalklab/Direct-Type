#ifndef DIRECTTYPE_CONFIG_H
#define DIRECTTYPE_CONFIG_H

#include <fcitx-config/configuration.h>
#include <fcitx-config/enum.h>
#include <fcitx-config/option.h>
#include <fcitx-utils/i18n.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/macros.h>

#include "viet.h"

namespace fcitx {

// In namespace fcitx so `Option`'s description dump finds the generated `Enum/N` entries,
// which is what makes these show up as combo boxes instead of text fields.
FCITX_CONFIG_ENUM(DirectTypeMethod, Telex, SimpleTelex, Vni);
FCITX_CONFIG_ENUM(DirectTypeToneStyle, Old, New);

} // namespace fcitx

namespace directtype {

/// Everything the addon remembers between sessions, and everything `fcitx5-configtool`
/// shows. The defaults match the engine's own (spec §2.1) so a fresh install types what the
/// engine is documented to type.
///
/// `vietnameseEnabled` is the language toggle: one input method, two states, so switching to
/// English does not mean choosing a different input method in fcitx5's list. `toggleKey`
/// defaults to Alt+Z: the UniKey habit, unused by fcitx5's own bindings, and pressable inside
/// a text field.
FCITX_CONFIGURATION(
    DirectTypeConfig,
    fcitx::Option<fcitx::DirectTypeMethod> method{this, "Method", _("Input method"),
                                                  fcitx::DirectTypeMethod::Telex};
    fcitx::Option<fcitx::DirectTypeToneStyle> toneStyle{
        this, "ToneStyle", _("Tone placement"), fcitx::DirectTypeToneStyle::New};
    fcitx::Option<bool> autoRestore{this, "AutoRestore",
                                    _("Restore a word that is not Vietnamese"), true};
    fcitx::Option<bool> spellCheck{this, "SpellCheck",
                                   _("Never transform a word that cannot become Vietnamese"),
                                   false};
    fcitx::Option<bool> freeToneMark{this, "FreeToneMark",
                                     _("Accept the tone key anywhere in a syllable"), true};
    fcitx::Option<bool> clientPreedit{
        this, "ClientPreedit",
        _("Draw the word in the text field instead of a separate preedit window"), true};
    fcitx::Option<bool> vietnameseEnabled{this, "VietnameseEnabled", _("Vietnamese"), true};
    fcitx::Option<fcitx::Key> toggleKey{this, "ToggleKey",
                                        _("Toggle Vietnamese/English"),
                                        fcitx::Key("Alt+Z")};
);

/// Translate the addon's options into the engine's C configuration. The numbers are the ones
/// `viet.h` documents, so this function is the single place where an option becomes an ABI
/// field — and the place to look when "VNI selected, Telex typed" is the bug report.
viet_config_t toEngineConfiguration(const DirectTypeConfig &config);

} // namespace directtype

#endif // DIRECTTYPE_CONFIG_H

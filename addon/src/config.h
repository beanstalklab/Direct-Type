#ifndef DIRECTTYPE_CONFIG_H
#define DIRECTTYPE_CONFIG_H

#include <fcitx-config/configuration.h>
#include <fcitx-config/option.h>
#include <fcitx-utils/i18n.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/macros.h>

namespace directtype {

/// Everything the addon remembers between sessions.
///
/// `vietnameseEnabled` is the language toggle: one input method, two states, so switching
/// to English does not mean choosing a different input method in fcitx5's list.
///
/// `toggleKey` defaults to Alt+Z: it is what UniKey users know, it does not collide with
/// fcitx5's own bindings, and a plain letter shortcut would be useless inside a text field.
/// T4.4 exposes both of these in `fcitx5-configtool`; until then they are edited in
/// `~/.config/fcitx5/conf/directtype.conf`.
FCITX_CONFIGURATION(
    DirectTypeConfig,
    fcitx::Option<bool> vietnameseEnabled{this, "VietnameseEnabled", _("Vietnamese"), true};
    fcitx::Option<fcitx::Key> toggleKey{this, "ToggleKey",
                                        _("Toggle Vietnamese/English"),
                                        fcitx::Key("Alt+Z")};
);

} // namespace directtype

#endif // DIRECTTYPE_CONFIG_H

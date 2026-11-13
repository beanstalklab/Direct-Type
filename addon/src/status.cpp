#include "status.h"

#include <fcitx-utils/i18n.h>

namespace directtype {

std::string statusLabel(bool vietnameseEnabled) {
    return vietnameseEnabled ? _("Vietnamese") : _("English");
}

} // namespace directtype

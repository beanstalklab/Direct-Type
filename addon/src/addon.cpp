#include "engine.h"

#include <fcitx/addonfactory.h>

namespace {

/// Creates the engine. fcitx5 instantiates one addon instance and asks it for its input
/// methods.
class DirectTypeAddonFactory final : public fcitx::AddonFactory {
public:
    fcitx::AddonInstance *create(fcitx::AddonManager *manager) override {
        (void)manager;
        return new DirectTypeEngine;
    }
};

} // namespace

// The symbol fcitx5 looks for in the module: `fcitx_addon_factory_instance`.
FCITX_ADDON_FACTORY(DirectTypeAddonFactory);

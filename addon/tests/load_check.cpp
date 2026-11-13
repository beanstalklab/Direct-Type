// Checks the contract fcitx5 relies on when it loads the addon: the module exports
// `fcitx_addon_factory_instance`, that factory builds an engine, and the engine lists
// exactly the one input method whose unique name the metadata points at.
//
// This is what can be verified without an fcitx5 session. That the entry then appears in
// `fcitx5-configtool` and types in a real application is a human check (T4.1 acceptance,
// T4.2).

#include <cstdio>
#include <memory>

#include <dlfcn.h>

#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/inputmethodengine.h>

namespace {

int failures = 0;

void check(bool condition, const char *message) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", message);
        failures += 1;
    }
}

} // namespace

int main(int argc, char **argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: %s <directtype.so>\n", argv[0]);
        return 2;
    }

    void *module = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (module == nullptr) {
        std::fprintf(stderr, "FAIL: dlopen: %s\n", dlerror());
        return 1;
    }

    auto factoryFunction =
        reinterpret_cast<fcitx::AddonFactory *(*)()>(dlsym(module, "fcitx_addon_factory_instance"));
    check(factoryFunction != nullptr,
          "the module exports fcitx_addon_factory_instance");
    if (factoryFunction != nullptr) {
        fcitx::AddonFactory *factory = factoryFunction();
        check(factory != nullptr, "the factory function returns a factory");
        if (factory != nullptr) {
            // The instance must be destroyed before the module is unloaded, because its
            // vtable lives in the module.
            std::unique_ptr<fcitx::AddonInstance> instance(factory->create(nullptr));
            check(instance != nullptr, "the factory creates an instance");

            auto *engine = dynamic_cast<fcitx::InputMethodEngine *>(instance.get());
            check(engine != nullptr, "the instance is an input method engine");
            // The status area asks for the sub-mode label through V2 and shows nothing for
            // a plain engine, so being a V2 is what makes the language visible at all.
            check(dynamic_cast<fcitx::InputMethodEngineV2 *>(engine) != nullptr,
                  "the engine is a V2, or the status area would show no language");
            if (engine != nullptr) {
                std::vector<fcitx::InputMethodEntry> entries = engine->listInputMethods();
                check(entries.size() == 1, "the engine offers exactly one input method");
                if (entries.size() == 1) {
                    const fcitx::InputMethodEntry &entry = entries.front();
                    check(entry.uniqueName() == "directtype",
                          "the input method is named directtype");
                    check(entry.languageCode() == "vi", "the input method is Vietnamese");
                    // fcitx5 routes an entry back to an engine by this name, so it has to
                    // match the addon's own name in directtype.conf.
                    check(entry.addon() == "directtype",
                          "the entry points back at this addon");
                }
            }
            instance.reset();
        }
    }

    dlclose(module);

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("addon load check passed\n");
    return 0;
}

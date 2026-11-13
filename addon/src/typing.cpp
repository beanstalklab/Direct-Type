#include "typing.h"

#include <string>

namespace directtype {
namespace {

viet_input_t characterInput(uint32_t codepoint) {
    viet_input_t input;
    input.kind = 0;
    input.code_point = codepoint;
    return input;
}

/// kind: 2 is `Input::Boundary` in `viet.h`.
viet_input_t boundaryInput() {
    viet_input_t input;
    input.kind = 2;
    input.code_point = 0;
    return input;
}

} // namespace

bool submit(viet_engine_t *engine, KeyAction action, uint32_t codepoint,
            TextTarget *target) {
    if (engine == nullptr) {
        return false;
    }

    const viet_input_t input = action == KeyAction::Character ? characterInput(codepoint)
                                                              : boundaryInput();
    viet_action_t result;
    const char *commit = nullptr;
    viet_engine_feed(engine, &input, &result, &commit, nullptr);

    if (target != nullptr) {
        if (result.backspaces > 0) {
            // Characters, not bytes: `ệ` is three bytes of UTF-8.
            target->deleteBeforeCaret(result.backspaces);
        }
        if (result.commit_length > 0 && commit != nullptr) {
            target->insert(std::string(commit, result.commit_length));
        }
    }

    return result.consumed != 0;
}

} // namespace directtype

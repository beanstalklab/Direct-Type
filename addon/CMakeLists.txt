cmake_minimum_required(VERSION 3.16)

# The addon version is the product version; the engine's crate version follows it.
project(directtype-addon VERSION 0.0.4 LANGUAGES CXX)

# fcitx5's package files default to `${CMAKE_INSTALL_PREFIX}/lib/fcitx5`, which on a Debian
# install is `/usr/local/lib/fcitx5` - a directory fcitx5 never looks in. Ask for the
# installed fcitx5's own directories instead; the two cache variables below still allow a
# per-user install to override them.
set(FCITX_INSTALL_USE_FCITX_SYS_PATHS On
    CACHE BOOL "Install into the directories of the installed fcitx5")

find_package(Fcitx5Core 5.1.7 REQUIRED)

# fcitx5 5.1.7's CMake package exports imported targets but no `fcitx5_addon()` helper, so
# the module and its metadata are declared and installed by hand. The two locations below
# come from `Fcitx5UtilsConfig.cmake` (`FCITX_INSTALL_ADDONDIR` is
# `/usr/lib/<multiarch>/fcitx5` on Debian). They are cache variables so a per-user install
# can point them at `~/.local` without patching fcitx5's absolute paths.
set(DIRECTTYPE_ADDON_DIR "${FCITX_INSTALL_ADDONDIR}"
    CACHE PATH "Where fcitx5 loads addon modules from")
set(DIRECTTYPE_DATA_DIR "${FCITX_INSTALL_PKGDATADIR}"
    CACHE PATH "Where fcitx5 reads addon metadata from")

# The engine is a Rust static library. `cargo` is invoked on every build so the module can
# never link a stale engine; cargo itself is a no-op when nothing changed.
find_program(CARGO_EXECUTABLE cargo REQUIRED)
set(ENGINE_ARCHIVE "${CMAKE_CURRENT_SOURCE_DIR}/../target/release/libviet.a")

add_custom_target(engine
    COMMAND "${CARGO_EXECUTABLE}" build --release --package viet-ffi
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/.."
    COMMENT "Building the Rust engine (viet-ffi)"
    VERBATIM
)

add_library(directtype MODULE
    src/addon.cpp
    src/config.cpp
    src/engine.cpp
    src/keys.cpp
    src/status.cpp
    src/typing.cpp
)
add_dependencies(directtype engine)
target_include_directories(directtype PRIVATE
    src
    "${CMAKE_CURRENT_SOURCE_DIR}/../crates/viet-ffi/include"
)
# A Rust static library brings its own standard library, which needs the C runtime's
# threads, dynamic loader and maths symbols on Linux.
target_link_libraries(directtype PRIVATE Fcitx5::Core "${ENGINE_ARCHIVE}" pthread dl m)
# Our own code, our own warnings: the addon is the first C++ in the repository, so it
# starts at the same bar as the Rust side.
target_compile_options(directtype PRIVATE -Wall -Wextra -Wpedantic -Werror)

# fcitx5 dlopen()s the module itself, so it must not carry the `lib` prefix.
set_target_properties(directtype PROPERTIES
    PREFIX ""
    OUTPUT_NAME "directtype"
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
)

configure_file("${CMAKE_CURRENT_SOURCE_DIR}/directtype.conf.in"
               "${CMAKE_CURRENT_BINARY_DIR}/directtype.conf" @ONLY)

install(TARGETS directtype
        DESTINATION "${DIRECTTYPE_ADDON_DIR}"
        COMPONENT addon)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/directtype.conf"
        DESTINATION "${DIRECTTYPE_DATA_DIR}/addon"
        COMPONENT config)

message(STATUS "directtype: module -> ${DIRECTTYPE_ADDON_DIR}/directtype.so")
message(STATUS "directtype: metadata -> ${DIRECTTYPE_DATA_DIR}/addon/directtype.conf")

# The one check that needs no session: dlopen the built module, call the factory fcitx5
# calls, and see that the engine lists the input method the metadata points at. Loading it
# into a running fcitx5 and typing is a human check.
enable_testing()
add_executable(addon_load_check tests/load_check.cpp)
target_link_libraries(addon_load_check PRIVATE Fcitx5::Core ${CMAKE_DL_LIBS})
target_compile_options(addon_load_check PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME addon_load_check COMMAND addon_load_check $<TARGET_FILE:directtype>)

# Which keystrokes are word characters is a product decision and does not need a session to
# check, so it is checked here rather than only by a human typing.
add_executable(key_mapping_check tests/key_mapping_check.cpp src/keys.cpp)
target_include_directories(key_mapping_check PRIVATE src)
target_link_libraries(key_mapping_check PRIVATE Fcitx5::Core)
target_compile_options(key_mapping_check PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME key_mapping_check COMMAND key_mapping_check)

# The typing path itself — classify, feed the engine, delete characters, commit — against a
# string instead of an input context. No session needed; the last mile stays a human check.
add_executable(typing_check tests/typing_check.cpp src/keys.cpp src/typing.cpp)
target_include_directories(typing_check PRIVATE
    src
    "${CMAKE_CURRENT_SOURCE_DIR}/../crates/viet-ffi/include"
)
target_link_libraries(typing_check PRIVATE Fcitx5::Core "${ENGINE_ARCHIVE}" pthread dl m)
target_compile_options(typing_check PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_dependencies(typing_check engine)
add_test(NAME typing_check COMMAND typing_check)

# The settings survive a restart, and they live where a restart looks for them.
add_executable(config_check tests/config_check.cpp
    src/config.cpp src/engine.cpp src/keys.cpp src/status.cpp src/typing.cpp)
target_include_directories(config_check PRIVATE
    src
    "${CMAKE_CURRENT_SOURCE_DIR}/../crates/viet-ffi/include"
)
target_link_libraries(config_check PRIVATE Fcitx5::Core "${ENGINE_ARCHIVE}" pthread dl m)
target_compile_options(config_check PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_dependencies(config_check engine)
add_test(NAME config_check COMMAND config_check)

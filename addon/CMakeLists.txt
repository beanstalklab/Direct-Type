cmake_minimum_required(VERSION 3.16)

# The addon version is the product version; the engine's crate version follows it.
project(directtype-addon VERSION 0.0.0 LANGUAGES CXX)

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

add_library(directtype MODULE
    src/addon.cpp
    src/engine.cpp
)
target_include_directories(directtype PRIVATE src)
target_link_libraries(directtype PRIVATE Fcitx5::Core)
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

#!/usr/bin/env bash
#
# Build a Debian package of the addon into `dist/`.
#
# The package is self-contained: the Rust engine is linked statically into the addon module,
# so the only runtime dependencies are fcitx5's own libraries and the C runtime. Those are
# computed from the built module with `dpkg-shlibdeps` instead of being written down by hand,
# because a wrong dependency is an addon that does not load.
#
# Debian/Ubuntu only, by design: the product targets Ubuntu, and this script refuses to guess
# a package format it cannot test.
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v dpkg-deb >/dev/null || ! command -v dpkg-shlibdeps >/dev/null; then
    echo "error: this script needs dpkg-deb and dpkg-shlibdeps (Debian or Ubuntu)" >&2
    exit 2
fi

version=$(sed -n 's/^project(directtype-addon VERSION \([0-9][0-9.]*\).*/\1/p' addon/CMakeLists.txt)
if [ -z "$version" ]; then
    echo "error: cannot read the version from addon/CMakeLists.txt" >&2
    exit 2
fi

architecture=$(dpkg-architecture -qDEB_HOST_ARCH)
multiarch=$(dpkg-architecture -qDEB_HOST_MULTIARCH)
addon_directory="/usr/lib/${multiarch}/fcitx5"
data_directory="/usr/share/fcitx5"

stage="build/deb/directtype_${version}_${architecture}"
rm -rf "$stage"
mkdir -p "$stage/DEBIAN"

# 1. The module and its metadata, installed straight into the staged tree.
cargo build --release --package viet-ffi
cmake -S addon -B build/deb/cmake -DCMAKE_BUILD_TYPE=Release \
      -DDIRECTTYPE_ADDON_DIR="$stage$addon_directory" \
      -DDIRECTTYPE_DATA_DIR="$stage$data_directory" >/dev/null
cmake --build build/deb/cmake >/dev/null
cmake --install build/deb/cmake >/dev/null

# 2. Documentation. Debian requires `copyright`; the licence texts travel with it.
documentation="$stage/usr/share/doc/directtype"
mkdir -p "$documentation"
install -m 0644 README.md "$documentation/README.md"
install -m 0644 LICENSE-MIT LICENSE-APACHE "$documentation/"
cat > "$documentation/copyright" <<'COPYRIGHT'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: directtype
Upstream-Contact: https://github.com/beanstalklab/Direct-Type
Source: https://github.com/beanstalklab/Direct-Type

Files: *
Copyright: 2026 beanstalklab
License: MIT or Apache-2.0
 This package is licensed under either of the MIT licence or the Apache License,
 Version 2.0, at your option. The full texts are in LICENSE-MIT and LICENSE-APACHE
 in this directory.
COPYRIGHT

{
    printf 'directtype (%s) unstable; urgency=low\n\n' "$version"
    printf '  * Package built from %s.\n\n' "$(git rev-parse --short HEAD 2>/dev/null || echo "a source tree")"
    printf ' -- beanstalklab <hung.beanstalk@gmail.com>  %s\n' "$(date -R)"
} | gzip -9n > "$documentation/changelog.gz"

# 3. Dependencies, from the binary. `dpkg-shlibdeps` wants a `debian/control` next to it, so
# it runs in a scratch directory and is handed an absolute path to the staged module.
staged_module="$(pwd)/$stage$addon_directory/directtype.so"
dependencies=$(
    mkdir -p build/deb/shlibdeps/debian
    printf 'Source: directtype\n\nPackage: directtype\nArchitecture: any\nDescription: directtype\n' \
        > build/deb/shlibdeps/debian/control
    cd build/deb/shlibdeps
    dpkg-shlibdeps -O -e"$staged_module" 2>/dev/null | sed -n 's/^shlibs:Depends=//p'
)
if [ -z "$dependencies" ]; then
    echo "error: dpkg-shlibdeps could not determine the dependencies" >&2
    exit 1
fi

# 4. Control file, then the package.
cat > "$stage/DEBIAN/control" <<CONTROL
Package: directtype
Version: $version
Architecture: $architecture
Maintainer: beanstalklab <hung.beanstalk@gmail.com>
Installed-Size: $(du -sk "$stage" | cut -f1)
Depends: $dependencies
Recommends: fcitx5
Section: utils
Priority: optional
Homepage: https://github.com/beanstalklab/Direct-Type
Description: Vietnamese input method for fcitx5 without a composition buffer
 Direct Type commits every keystroke straight into the application instead of
 building a preedit string, so nothing is underlined and no spacebar is needed
 to finish a word. Telex, VNI and Simple Telex are supported, with auto-restore,
 spell-check, and old and new tone placement.
CONTROL

mkdir -p dist
package="dist/directtype_${version}_${architecture}.deb"
dpkg-deb --build --root-owner-group "$stage" "$package" >/dev/null
echo "built $package"
dpkg-deb --info "$package"

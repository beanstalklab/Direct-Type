#!/usr/bin/env bash
#
# Install Direct Type: a Vietnamese input method for fcitx5.
#
# The addon is a system input-method module, so installing needs root — the same as any
# distribution package. Two ways in:
#
#   * a prebuilt package in `dist/` (built with `scripts/build_deb.sh`, or downloaded),
#   * from source, when this machine has the toolchain (cargo, cmake, g++, fcitx5 headers).
#
# Either way the result is the same Debian package, so there is one install path to test and
# one thing to uninstall. The script refuses to guess when a prerequisite is missing: it names
# the package to install and stops, because an installer that half-succeeds is worse than one
# that says "install fcitx5 first".
set -euo pipefail

cd "$(dirname "$0")"

readonly package_name="directtype"
readonly source_packages="cargo cmake g++ libfcitx5core-dev dpkg-dev"
readonly fcitx_packages="fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt5"

usage() {
    cat <<EOF
Usage: ./install.sh [--package FILE | --from-source | --uninstall | --help]

  (no arguments)   install the newest package in dist/, or build one first if there is none
  --package FILE   install a specific .deb
  --from-source    build the package from this tree, even if dist/ has one
  --uninstall      remove Direct Type
  --help           show this text

After installing, add "Direct Type" to fcitx5's input method list if it is not there yet
(fcitx5-configtool). Alt+Z switches between Vietnamese and English; the settings live in
fcitx5-configtool under Direct Type.
EOF
}

die() {
    echo "error: $*" >&2
    exit 1
}

require_fcitx5() {
    if ! command -v fcitx5 >/dev/null 2>&1; then
        echo "error: fcitx5 is not installed, and Direct Type is an fcitx5 addon." >&2
        echo "       Install it first: sudo apt install ${fcitx_packages}" >&2
        echo "       then set fcitx5 as your input method and run this script again." >&2
        exit 1
    fi
}

build_package() {
    for command_name in cargo cmake g++ dpkg-deb; do
        if ! command -v "$command_name" >/dev/null 2>&1; then
            die "building from source needs ${source_packages}, but ${command_name} is missing"
        fi
    done
    if ! pkg-config --exists Fcitx5Core 2>/dev/null; then
        die "building from source needs the fcitx5 headers: ${source_packages}"
    fi
    echo "building the package from source (this takes a minute)"
    bash scripts/build_deb.sh
}

install_package() {
    local package="$1"
    [ -f "$package" ] || die "no such package: $package"
    echo "installing ${package}"
    # `apt` resolves the dependencies from the distribution's repositories, which `dpkg -i`
    # alone would not. `--reinstall` makes the script idempotent: running it twice must not
    # fail, and it is also how an upgraded package replaces the installed one.
    sudo apt-get install -y --reinstall "$package"
}

restart_fcitx5() {
    if command -v fcitx5-remote >/dev/null 2>&1; then
        fcitx5-remote -r >/dev/null 2>&1 || true
    fi
}

case "${1:-}" in
--help | -h)
    usage
    ;;
--uninstall)
    echo "removing ${package_name}"
    sudo apt-get remove -y "$package_name"
    restart_fcitx5
    echo
    echo "If 'Direct Type' is still in your fcitx5 input method list, remove it there too."
    ;;
--package)
    [ $# -ge 2 ] || die "--package needs a file"
    require_fcitx5
    install_package "$2"
    restart_fcitx5
    ;;
--from-source)
    require_fcitx5
    build_package
    install_package "$(ls -1 dist/directtype_*.deb | sort -V | tail -1)"
    restart_fcitx5
    ;;
"")
    require_fcitx5
    package=$(ls -1 dist/directtype_*.deb 2>/dev/null | sort -V | tail -1 || true)
    if [ -z "$package" ]; then
        build_package
        package=$(ls -1 dist/directtype_*.deb | sort -V | tail -1)
    fi
    install_package "$package"
    restart_fcitx5
    ;;
*)
    usage >&2
    die "unknown option: $1"
    ;;
esac

case "${1:-}" in
--help | -h | --uninstall) ;;
*)
    echo
    echo "Installed. Last step, once:"
    echo "  1. open fcitx5 configuration (fcitx5-configtool)"
    echo "  2. add 'Direct Type' to the input method list and press Apply"
    echo
    echo "Then type: Alt+Z switches Vietnamese/English. 'vieejt' becomes 'việt'."
    echo "If it does not appear, log out and back in so fcitx5 loads the new addon."
    ;;
esac

#!/usr/bin/env bash
#
# Build both library artifacts, check that the shared one exports the ABI, then compile
# and run the C smoke test against the static library and again against the shared one.
#
# This is the check `cargo test` cannot make: that `viet.h` and the Rust implementation
# agree from a caller that is not Rust. Run it from anywhere; it finds the repository root
# itself.
set -euo pipefail

cd "$(dirname "$0")/.."

cargo build -p viet-ffi --release

static_library=target/release/libviet.a
shared_library=target/release/libviet.so
for artifact in "$static_library" "$shared_library"; do
  if [ ! -f "$artifact" ]; then
    echo "error: $artifact was not built" >&2
    exit 1
  fi
done

# Every function the header declares has to be reachable from a dynamic caller.
for symbol in viet_engine_new viet_engine_free viet_engine_reset viet_engine_set_config \
              viet_engine_feed; do
  if ! nm -D "$shared_library" | grep -q " T ${symbol}$"; then
    echo "error: ${shared_library} does not export ${symbol}" >&2
    exit 1
  fi
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

compile_flags=(-std=c11 -Wall -Wextra -Werror -I crates/viet-ffi/include)
source_file=crates/viet-ffi/tests/smoke.c

echo "linking against ${static_library}"
cc "${compile_flags[@]}" "$source_file" "$static_library" \
  -o "$work/smoke-static" -lpthread -ldl -lm
"$work/smoke-static"

echo "linking against ${shared_library}"
cc "${compile_flags[@]}" "$source_file" -L target/release -lviet \
  -o "$work/smoke-shared"
LD_LIBRARY_PATH=target/release "$work/smoke-shared"

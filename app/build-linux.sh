#!/usr/bin/env bash
# Builds the Linux .deb and .rpm packages: TorminCore against the system libtorrent,
# then the Tauri app. Run from anywhere; on WSL prefer a copy inside the Linux
# file system (see AGENTS.md) - building on /mnt/c or /mnt/d is very slow.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
command -v cargo >/dev/null || { echo "Rust is missing: curl https://sh.rustup.rs -sSf | sh"; exit 1; }

# 1. Native core
cd "$root/core"
cmake --preset linux-release
cmake --build --preset linux-release -j"$(nproc)"

# libTorminCore.so finds libtorrent next to itself (rpath $ORIGIN); copy it there
# under its soname so the bundle carries it and works without libtorrent installed.
out="$root/core/build/linux-release"
lib="$(ldd "$out/libTorminCore.so" | awk '/libtorrent-rasterbar/ {print $3}')"
[ -n "$lib" ] || { echo "libtorrent-rasterbar not found by ldd"; exit 1; }
# A rebuild finds the copy itself (through $ORIGIN): nothing to do then
[ "$(realpath "$lib")" = "$(realpath "$out/libtorrent-rasterbar.so.2.0" 2>/dev/null)" ] || cp -L "$lib" "$out/libtorrent-rasterbar.so.2.0"

# 2. App
cd "$root/app"
[ -d node_modules ] || npm ci
npm run tauri build

ls -lh "$root"/app/src-tauri/target/release/bundle/{deb/*.deb,rpm/*.rpm}

#!/bin/sh
# Builds the commands the browser runs by itself (web-wasm) into web/wasm/,
# where the stand-in for Tauri's `invoke` loads them. Needs the wasm32
# target (rustup target add wasm32-unknown-unknown) and wasm-bindgen of the
# version Cargo.lock has: on the PATH, or in target/tools as
#   cargo install --root target/tools wasm-bindgen-cli --version <it> --locked
set -eu
app=$(cd "$(dirname "$0")/.." && pwd)
root=$(dirname "$app")
PATH="$root/target/tools/bin:$PATH"
cargo build --quiet --release -p dactile-wasm --target wasm32-unknown-unknown --manifest-path "$root/Cargo.toml"
wasm-bindgen --target web --out-dir "$app/web/wasm" \
  "$root/target/wasm32-unknown-unknown/release/dactile_wasm.wasm"

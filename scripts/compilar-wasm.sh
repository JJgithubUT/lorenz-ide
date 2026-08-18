#!/bin/sh
# Compila el nucleo de Lorenz (lorenz-arch, lorenz-emu) a un modulo WebAssembly (RF-ECO-029).
# Requiere el objetivo wasm32 (`rustup target add wasm32-unknown-unknown`). No se ha podido
# verificar en el entorno de desarrollo; ver docs/WEB.md.
set -eu
cd "$(dirname "$0")/.."
rustup target list --installed | grep -q wasm32-unknown-unknown || {
    echo "falta el objetivo: rustup target add wasm32-unknown-unknown" >&2
    exit 1
}
cargo build --release --target wasm32-unknown-unknown -p lorenz-arch -p lorenz-emu
ls -l target/wasm32-unknown-unknown/release/*.rlib

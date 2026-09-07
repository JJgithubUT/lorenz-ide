#!/bin/sh
# Compila el nucleo de Lorenz a WebAssembly (RF-ECO-029) y deja el modulo junto a su pagina de prueba.
# Requiere: rustup target add wasm32-unknown-unknown
set -eu
cd "$(dirname "$0")/.."
rustup target list --installed | grep -q wasm32-unknown-unknown || {
    echo "falta el objetivo: rustup target add wasm32-unknown-unknown" >&2
    exit 1
}
cargo build --release --target wasm32-unknown-unknown -p lorenz-wasm
cp target/wasm32-unknown-unknown/release/lorenz_wasm.wasm crates/lorenz-wasm/web/
echo "Listo: sirve crates/lorenz-wasm/web con cualquier servidor estatico y abre index.html"

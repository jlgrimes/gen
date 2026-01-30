#!/bin/bash
set -e

# Always install rustup to ensure we have wasm32 target support
# Vercel's pre-installed Rust doesn't have rustup, so we need our own
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
. "$CARGO_HOME/env"

# Add wasm32 target
rustup target add wasm32-unknown-unknown

# Install wasm-pack if not present
if ! command -v wasm-pack &> /dev/null; then
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build WASM package
cd ../gen-wasm
wasm-pack build --target web

# Install npm dependencies
cd ../gen-web
pnpm install

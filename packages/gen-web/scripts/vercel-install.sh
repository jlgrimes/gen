#!/bin/bash
set -e

# Set up Rust environment
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"

# Check if rustup is available, if not install it
if ! command -v rustup &> /dev/null; then
    # Install rustup (use -y to skip confirmation, and handle existing installations)
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --no-modify-path
fi

# Source cargo env if it exists
if [ -f "$CARGO_HOME/env" ]; then
    . "$CARGO_HOME/env"
fi

# Add wasm32 target (ignore if already installed)
rustup target add wasm32-unknown-unknown 2>/dev/null || true

# Install wasm-pack if not present
if ! command -v wasm-pack &> /dev/null; then
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build WASM package
cd ../gen-wasm
wasm-pack build --target web

# Install npm dependencies (production only to skip canvas)
cd ../gen-web
pnpm install --prod

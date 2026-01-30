#!/bin/bash
set -e

# Set default toolchain for Vercel's pre-installed rustup
rustup default stable

# Add wasm32 target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM package
cd ../gen-wasm
wasm-pack build --target web

# Install npm dependencies
cd ../gen-web
pnpm install

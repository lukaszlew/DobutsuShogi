#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

[ -d "$HOME/.cargo/bin" ] && export PATH="$HOME/.cargo/bin:$PATH"

missing=0
check() {
  local name=$1 hint=$2
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "missing: $name"
    echo "  install: $hint"
    missing=1
  fi
}

check node       "https://nodejs.org/ (or your package manager)"
check npm        "ships with node"
check cargo      "https://rustup.rs/"

check cargo-quickinstall "cargo install cargo-quickinstall"
check wasm-pack          "cargo quickinstall wasm-pack"
check cargo-watch        "cargo quickinstall cargo-watch"

[ "$missing" -eq 0 ] || exit 1

cd web
[ -d node_modules ] || npm install
exec npm run dev

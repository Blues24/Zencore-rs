#!/bin/bash
targets=(
  x86_64-unknown-linux-gnu
  x86_64-pc-windows-gnu
  aarch64-unknown-linux-gnu
)

for target in "${targets[@]}"; do
  echo "🔨 Building for $target..."
  cargo build --target "$target" --release
done

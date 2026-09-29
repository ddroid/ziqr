#!/bin/sh
set -eu
root="$1"
output="$2"
target_dir="$3"
cargo build --release --manifest-path "$root/src_rust/Cargo.toml" --target-dir "$target_dir"
cp "$target_dir/release/eeman" "$output"

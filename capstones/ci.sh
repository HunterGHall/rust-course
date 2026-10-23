#!/usr/bin/env bash
# Builds, tests and smoke-tests every capstone. Used by CI; handy locally too.
set -euo pipefail
cd "$(dirname "$0")"

rustc --version
cargo --version

echo "== task-tracker =="
(
  cd task-tracker
  cargo build
  export TASKS_FILE="$PWD/smoke-$$.txt"
  trap 'rm -f "$TASKS_FILE"' EXIT
  cargo run -q -- add "Buy milk"
  cargo run -q -- add "Learn Rust"
  cargo run -q -- done 2
  cargo run -q -- list
  if cargo run -q -- remove 9; then echo "expected 'remove 9' to fail"; exit 1; fi
)

echo "== link-checker =="
(
  cd link-checker
  cargo build
  cargo test
)

echo "== file-indexer =="
(
  cd file-indexer
  cargo build
  cargo test
)

echo "All capstones OK"

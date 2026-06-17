#!/usr/bin/env bash
# Run all more-pit tests (compile tests skip when optional compilers are missing).
set -euo pipefail
cd "$(dirname "$0")/.."
echo "==> cargo test --workspace"
cargo test --workspace
echo "==> pit-gen compile tests"
cargo test -p pit-gen -- --nocapture
echo "==> done"

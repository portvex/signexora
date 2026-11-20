#!/usr/bin/env bash
# Run the same checks as CI: formatting, lints, tests.
# Usage: scripts/check.sh            (check only)
#        scripts/check.sh --fix      (format and apply clippy fixes first)
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "${1:-}" == "--fix" ]]; then
  echo "==> cargo fmt"
  cargo fmt --all
  echo "==> cargo clippy --fix"
  cargo lintfix
fi

echo "==> format check"
cargo fmtcheck
echo "==> clippy (warnings are errors)"
cargo lint
echo "==> tests"
cargo test --workspace --all-targets
echo "==> doc tests"
cargo test --workspace --doc
echo "All checks passed."

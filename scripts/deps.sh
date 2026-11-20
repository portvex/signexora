#!/usr/bin/env bash
# Dependency checks: known vulnerabilities (cargo audit) and licences, bans,
# advisories and sources (cargo deny, configured in deny.toml).
# Install the tools once with: cargo install --locked cargo-audit cargo-deny
set -euo pipefail
cd "$(dirname "$0")/.."

missing=0
for tool in cargo-audit cargo-deny; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: $tool is not installed (cargo install --locked $tool)" >&2
    missing=1
  fi
done
[[ $missing -eq 0 ]] || exit 1

echo "==> cargo audit"
cargo audit
echo "==> cargo deny"
cargo deny --all-features check
echo "Dependency checks passed."

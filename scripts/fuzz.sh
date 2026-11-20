#!/usr/bin/env bash
# Fuzz the parsers and DSP entry points with libFuzzer (cargo-fuzz).
#
# Usage: scripts/fuzz.sh                 run every target for 60 s each
#        scripts/fuzz.sh edit 600        run one target for 600 s
#        scripts/fuzz.sh --list          list the targets
#
# Needs a nightly toolchain and cargo-fuzz:
#   rustup toolchain install nightly && cargo install --locked cargo-fuzz
# Crashes are saved under fuzz/artifacts/<target>/; reproduce one with
#   scripts/fuzz.sh <target> 0 fuzz/artifacts/<target>/crash-...
set -euo pipefail
cd "$(dirname "$0")/../fuzz"

# Use rustup's nightly even when another cargo/rustc (for example Homebrew's)
# comes first on PATH.
nightly_bin="$(dirname "$(rustup which --toolchain nightly rustc)")"
export PATH="$nightly_bin:$HOME/.cargo/bin:$PATH"

targets=(open_file sigmf session_json edit ofdm)
if [[ "${1:-}" == "--list" ]]; then
  printf '%s\n' "${targets[@]}"
  exit 0
fi

run() {
  local target=$1 seconds=$2
  shift 2
  mkdir -p "corpus/$target"
  if [[ -d "seeds/$target" ]]; then
    cp -n seeds/"$target"/* "corpus/$target/" 2>/dev/null || true
  fi
  echo "==> fuzz $target (${seconds} s)"
  if [[ $# -gt 0 ]]; then
    cargo fuzz run "$target" "$@"
  else
    # rss_limit_mb catches runaway allocations; -timeout catches hangs.
    cargo fuzz run "$target" "corpus/$target" -- \
      -max_total_time="$seconds" -rss_limit_mb=2048 -timeout=20
  fi
}

if [[ $# -ge 1 ]]; then
  target=$1
  seconds=${2:-60}
  shift $(( $# >= 2 ? 2 : 1 ))
  run "$target" "$seconds" "$@"
else
  for t in "${targets[@]}"; do
    run "$t" 60
  done
fi

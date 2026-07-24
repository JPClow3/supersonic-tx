#!/usr/bin/env bash
set -uo pipefail
echo "=== full suite $(date -Iseconds) ==="
echo "=== cargo fmt --check ==="
rustup component add rustfmt 2>&1 || true
cargo fmt --all -- --check
FMT_EC=$?
echo "FMT_EXIT=$FMT_EC"
echo "=== cargo test --workspace --locked ==="
cargo test --workspace --locked
TEST_EC=$?
echo "WORKSPACE_TEST_EXIT=$TEST_EC"
echo "=== router tests programs/supersonic-tx-tests ==="
if [ -d programs/supersonic-tx-tests ]; then
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR%/}/program-tests"
  fi
  cargo test --locked --manifest-path programs/supersonic-tx-tests/Cargo.toml
  ROUTER_EC=$?
else
  echo "router tests dir missing"
  ROUTER_EC=1
fi
echo "ROUTER_TEST_EXIT=$ROUTER_EC"
echo "=== SUMMARY fmt=$FMT_EC workspace=$TEST_EC router=$ROUTER_EC ==="
if [ "$FMT_EC" -ne 0 ] || [ "$TEST_EC" -ne 0 ] || [ "$ROUTER_EC" -ne 0 ]; then
  exit 1
fi
exit 0
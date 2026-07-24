#!/usr/bin/env bash
set -euo pipefail
cd /workspace
restore() {
  [ -f Cargo.toml.host.bak ] && mv -f Cargo.toml.host.bak Cargo.toml
  [ -f Cargo.lock.host.bak ] && mv -f Cargo.lock.host.bak Cargo.lock
}
trap restore EXIT
cp Cargo.toml Cargo.toml.host.bak
cp Cargo.lock Cargo.lock.host.bak
cp .superpowers/sdd/briefs/Cargo.lock.sbf.v3 Cargo.lock
awk '
/^members = \[/ { print "members = ["; print "    \"programs/supersonic-tx\","; print "    \"crates/supersonic-tx-core\","; print "]"; skip=1; next }
skip && /^\]/ { skip=0; next }
!skip { print }
' Cargo.toml.host.bak > Cargo.toml
cargo metadata --format-version 1 --locked >/dev/null
echo METADATA_OK
anchor build 2>&1 | tee .superpowers/sdd/briefs/bar-c-anchor-build-closeout-2026-07-23.log
ANCHOR_EC=${PIPESTATUS[0]}
mkdir -p target/deploy target/idl target/types
cp -f /workspace-target/deploy/supersonic_tx.so target/deploy/ 2>/dev/null || true
cp -f /workspace-target/deploy/supersonic_tx-keypair.json target/deploy/ 2>/dev/null || true
cp -f /workspace-target/idl/supersonic_tx.json target/idl/ 2>/dev/null || true
cp -f /workspace-target/types/supersonic_tx.ts target/types/ 2>/dev/null || true
echo ANCHOR_EXIT:$ANCHOR_EC
ls -la target/deploy/ target/idl/ 2>/dev/null || true
exit $ANCHOR_EC
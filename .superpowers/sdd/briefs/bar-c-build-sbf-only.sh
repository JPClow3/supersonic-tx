#!/usr/bin/env bash
set -euo pipefail
cd /workspace
restore(){ [ -f Cargo.toml.host.bak ] && mv -f Cargo.toml.host.bak Cargo.toml; [ -f Cargo.lock.host.bak ] && mv -f Cargo.lock.host.bak Cargo.lock; }
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
cargo build-sbf --manifest-path programs/supersonic-tx/Cargo.toml -- --locked
echo BUILD_SBF_OK
ls -la /workspace-target/deploy/
anchor idl build -p supersonic_tx 2>&1 | tail -5
ls -la target/idl/ 2>/dev/null || ls -la /workspace-target/idl/ 2>/dev/null || true
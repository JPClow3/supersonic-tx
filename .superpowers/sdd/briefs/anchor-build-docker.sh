#!/usr/bin/env bash
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
export PATH="/root/.cargo/bin:/root/.local/share/solana/install/active_release/bin:/root/.avm/bin:$PATH"
if ! command -v solana >/dev/null 2>&1; then
  apt-get update -qq
  apt-get install -y -qq curl build-essential pkg-config libudev-dev libssl-dev ca-certificates git
  curl -sSfL https://release.anza.xyz/v1.18.26/install -o /tmp/solana-install.sh
  bash /tmp/solana-install.sh
  export PATH="/root/.local/share/solana/install/active_release/bin:$PATH"
fi
if ! command -v avm >/dev/null 2>&1; then
  cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
  export PATH="/root/.cargo/bin:$PATH"
fi
if ! command -v anchor >/dev/null 2>&1 || ! anchor --version 2>/dev/null | grep -q 0.30.1; then
  avm install 0.30.1
  avm use 0.30.1
  export PATH="/root/.avm/bin:$PATH"
fi
solana --version
anchor --version
anchor build
ls -la target/deploy/ 2>/dev/null || ls -la /workspace-target/deploy/ 2>/dev/null || true
find . -path '*supersonic_tx.so' 2>/dev/null | head -5

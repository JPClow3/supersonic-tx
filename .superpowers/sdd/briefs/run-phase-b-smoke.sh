#!/usr/bin/env bash
# Phase B localnet refresh: extra airdrop → cook → cast --via-router --send
# Ephemeral keys under .tmp-operator-sim/ (gitignored). Do not commit secrets.
set -uo pipefail

RPC="${RPC:-http://127.0.0.1:8899}"
WORKDIR="${WORKDIR:-/workspace}"
EPHEM="${WORKDIR}/.tmp-operator-sim"
COOKED="${EPHEM}/cooked"
CLI="${CARGO_TARGET_DIR:-/workspace-target}/release/supersonic-tx"
DEPLOYER="${EPHEM}/deployer.json"
TARGET="So11111111111111111111111111111111111111112"
PROG_ID="GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9"
REPORT="${EPHEM}/phase-b-results.env"

mkdir -p "$EPHEM" "$COOKED"
: > "$REPORT"

echo "=== phase-b smoke $(date -u +%Y-%m-%dT%H:%M:%SZ) RPC=$RPC ==="
[ -x "$CLI" ] || { echo "ERROR: CLI missing at $CLI"; exit 2; }
[ -f "$DEPLOYER" ] || { echo "ERROR: deployer missing at $DEPLOYER (run run-realworld-sim.sh first)"; exit 3; }

{
  echo "COMMIT=$(git -C "$WORKDIR" rev-parse HEAD)"
  echo "UTC=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "GENESIS=$(solana genesis-hash --url "$RPC")"
  echo "PROGRAM_ID=$PROG_ID"
  echo "DEPLOYER_PUBKEY=$(solana-keygen pubkey "$DEPLOYER")"
} | tee -a "$REPORT"

echo "=== program show ==="
SHOW_OUT=$(solana program show "$PROG_ID" --url "$RPC" --keypair "$DEPLOYER" 2>&1) || true
echo "$SHOW_OUT"
echo "$SHOW_OUT" > "${EPHEM}/program-show.txt"
if echo "$SHOW_OUT" | grep -qiE 'Executable|BPFLoader'; then
  echo "PROGRAM_OK=1" | tee -a "$REPORT"
else
  echo "PROGRAM_OK=0" | tee -a "$REPORT"
  exit 5
fi

echo "=== extra airdrop (post-deploy funding) ==="
AIR2_OUT=$(solana airdrop 10 --url "$RPC" --keypair "$DEPLOYER" 2>&1) || true
echo "$AIR2_OUT"
AIR2_SIG=$(echo "$AIR2_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 || true)
echo "AIRDROP2_SIG=${AIR2_SIG:-}" | tee -a "$REPORT"
solana balance --url "$RPC" --keypair "$DEPLOYER"
sleep 2

echo "=== deployer transaction history (deploy sig) ==="
HIST_OUT=$(solana transaction-history "$(solana-keygen pubkey "$DEPLOYER")" --url "$RPC" -n 15 2>&1) || true
echo "$HIST_OUT" | tee "${EPHEM}/history.out"
# Prefer a Signature: line if present; otherwise first base58-looking token after deploy era
DEPLOY_SIG=$(echo "$HIST_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 || true)
echo "DEPLOY_SIG_CANDIDATE=${DEPLOY_SIG:-}" | tee -a "$REPORT"

echo "=== cook ==="
rm -rf "$COOKED"
mkdir -p "$COOKED"
COOK_EC=0
COOK_OUT=$("$CLI" cook --sponsor-keypair "$DEPLOYER" --out-dir "$COOKED" --rpc-url "$RPC" --cluster localnet 2>&1) || COOK_EC=$?
echo "$COOK_OUT"
echo "COOK_EC=$COOK_EC" | tee -a "$REPORT"
HANDOFF=$(ls -1 "$COOKED"/handoff-*.json 2>/dev/null | head -1 || true)
echo "HANDOFF=${HANDOFF:-}" | tee -a "$REPORT"
[ -n "$HANDOFF" ] || { echo "ERROR: no handoff"; cat "$REPORT"; exit 4; }

echo "=== simulate ==="
SIM_EC=0
SIM_OUT=$("$CLI" simulate --handoff "$HANDOFF" --target "$TARGET" --amount 100000 --rpc-url "$RPC" --via-router 2>&1) || SIM_EC=$?
echo "$SIM_OUT"
echo "SIMULATE_EC=$SIM_EC" | tee -a "$REPORT"

echo "=== cast --via-router --send ==="
CAST_EC=0
CAST_OUT=$("$CLI" cast --handoff "$HANDOFF" --target "$TARGET" --amount 100000 --rpc-url "$RPC" --via-router --send 2>&1) || CAST_EC=$?
echo "$CAST_OUT" | tee "${EPHEM}/cast.out"
echo "CAST_EC=$CAST_EC" | tee -a "$REPORT"
CAST_SIG=$(echo "$CAST_OUT" | grep -oiE '(signature|sig)[=:[:space:]]+[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' || true)
if [ -z "${CAST_SIG:-}" ]; then
  CAST_SIG=$(echo "$CAST_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 || true)
fi
echo "CAST_SIG=${CAST_SIG:-}" | tee -a "$REPORT"

BYTES=$(echo "$CAST_OUT" | grep -oE '[0-9]+/1232' | head -1 || true)
DECOYS=$(echo "$CAST_OUT" | grep -oiE 'decoy[s]?[[:space:]]*[:=]?[[:space:]]*[0-9]+' | head -1 || true)
echo "CAST_BYTES=${BYTES:-}" | tee -a "$REPORT"
echo "CAST_DECOYS=${DECOYS:-}" | tee -a "$REPORT"

if [ -n "${CAST_SIG:-}" ]; then
  echo "=== confirm cast ==="
  solana confirm "$CAST_SIG" --url "$RPC" 2>&1 || true
fi

echo "=== phase-b results.env ==="
cat "$REPORT"
echo "=== phase-b done ==="
exit "$CAST_EC"

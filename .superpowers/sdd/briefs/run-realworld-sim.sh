#!/usr/bin/env bash
set -uo pipefail
RPC="${RPC:-http://host.docker.internal:8899}"
WORKDIR="${WORKDIR:-/workspace}"
EPHEM="${WORKDIR}/.tmp-operator-sim"
COOKED="${EPHEM}/cooked"
CLI="${CARGO_TARGET_DIR:-/workspace-target}/release/supersonic-tx"
SO="${WORKDIR}/target/deploy/supersonic_tx.so"
PROG_KP="${WORKDIR}/target/deploy/supersonic_tx-keypair.json"
PROG_ID="GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9"
TARGET="So11111111111111111111111111111111111111112"
REPORT_VARS="${EPHEM}/results.env"

mkdir -p "$EPHEM" "$COOKED"
: > "$REPORT_VARS"

echo "=== realworld sim $(date -Iseconds) RPC=$RPC ==="
command -v solana >/dev/null || { echo "ERROR: solana CLI missing"; exit 2; }
command -v solana-keygen >/dev/null || { echo "ERROR: solana-keygen missing"; exit 2; }
[ -x "$CLI" ] || { echo "ERROR: CLI missing at $CLI"; exit 2; }
[ -f "$SO" ] || { echo "ERROR: missing $SO"; exit 3; }
[ -f "$PROG_KP" ] || { echo "ERROR: missing $PROG_KP"; exit 3; }

DEPLOYER="${EPHEM}/deployer.json"
rm -f "$DEPLOYER"
solana-keygen new --no-bip39-passphrase --silent -o "$DEPLOYER"
echo "DEPLOYER_PUBKEY=$(solana-keygen pubkey "$DEPLOYER")" | tee -a "$REPORT_VARS"

solana config set --url "$RPC" --keypair "$DEPLOYER" >/dev/null

echo "=== airdrop ==="
AIRDROP_OUT=$(solana airdrop 10 --url "$RPC" --keypair "$DEPLOYER" 2>&1) || true
echo "$AIRDROP_OUT"
AIRDROP_SIG=$(echo "$AIRDROP_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 || true)
echo "AIRDROP_SIG=${AIRDROP_SIG:-}" | tee -a "$REPORT_VARS"
solana balance --url "$RPC" --keypair "$DEPLOYER"

echo "=== program show (pre) ==="
solana program show "$PROG_ID" --url "$RPC" 2>&1 || true

echo "=== deploy ==="
DEPLOY_OUT=$(solana program deploy "$SO" --program-id "$PROG_KP" --url "$RPC" --keypair "$DEPLOYER" 2>&1) || true
echo "$DEPLOY_OUT"
DEPLOY_SIG=$(echo "$DEPLOY_OUT" | grep -oiE 'Signature:[[:space:]]*[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 | awk '{print $NF}')
if [ -z "${DEPLOY_SIG:-}" ]; then
  DEPLOY_SIG=$(echo "$DEPLOY_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 || true)
fi
echo "DEPLOY_SIG=${DEPLOY_SIG:-}" | tee -a "$REPORT_VARS"

echo "=== program show (post) ==="
SHOW_OUT=$(solana program show "$PROG_ID" --url "$RPC" 2>&1) || true
echo "$SHOW_OUT"
if echo "$SHOW_OUT" | grep -qi 'Executable'; then
  echo "PROGRAM_EXECUTABLE=1" | tee -a "$REPORT_VARS"
else
  echo "PROGRAM_EXECUTABLE=0" | tee -a "$REPORT_VARS"
fi

echo "=== cook ==="
rm -rf "$COOKED"
mkdir -p "$COOKED"
COOK_EC=0
COOK_OUT=$("$CLI" cook --sponsor-keypair "$DEPLOYER" --out-dir "$COOKED" --rpc-url "$RPC" --cluster localnet 2>&1) || COOK_EC=$?
echo "$COOK_OUT"
echo "COOK_EC=$COOK_EC" | tee -a "$REPORT_VARS"
HANDOFF=$(ls -1 "$COOKED"/handoff-*.json 2>/dev/null | head -1 || true)
echo "HANDOFF=${HANDOFF:-}" | tee -a "$REPORT_VARS"
[ -n "$HANDOFF" ] || { echo "ERROR: no handoff"; cat "$REPORT_VARS"; exit 4; }

echo "=== simulate ==="
SIM_EC=0
SIM_OUT=$("$CLI" simulate --handoff "$HANDOFF" --target "$TARGET" --amount 100000 --rpc-url "$RPC" --via-router 2>&1) || SIM_EC=$?
echo "$SIM_OUT"
echo "SIMULATE_EC=$SIM_EC" | tee -a "$REPORT_VARS"

echo "=== cast --send ==="
CAST_EC=0
CAST_OUT=$("$CLI" cast --handoff "$HANDOFF" --target "$TARGET" --amount 100000 --rpc-url "$RPC" --via-router --send 2>&1) || CAST_EC=$?
echo "$CAST_OUT"
echo "CAST_EC=$CAST_EC" | tee -a "$REPORT_VARS"
CAST_SIG=$(echo "$CAST_OUT" | grep -oiE '(signature|sig)[=:[:space:]]+[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' || true)
if [ -z "${CAST_SIG:-}" ]; then
  CAST_SIG=$(echo "$CAST_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 || true)
fi
echo "CAST_SIG=${CAST_SIG:-}" | tee -a "$REPORT_VARS"

echo "=== campaign short --send ==="
CAMP_EC=0
CAMP_OUT=$("$CLI" campaign --handoff "$HANDOFF" --target "$TARGET" --amount 50000 --rpc-url "$RPC" --txs 2 --isolate-intent true --send 2>&1) || CAMP_EC=$?
echo "$CAMP_OUT"
echo "CAMPAIGN_EC=$CAMP_EC" | tee -a "$REPORT_VARS"
CAMP_SIGS=$(echo "$CAMP_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | tr '\n' ',' || true)
echo "CAMPAIGN_SIGS=${CAMP_SIGS:-}" | tee -a "$REPORT_VARS"

echo "=== negative: cast --send without keypair/handoff ==="
NEG1_EC=0
NEG1_OUT=$("$CLI" cast --target "$TARGET" --amount 100000 --rpc-url "$RPC" --send 2>&1) || NEG1_EC=$?
echo "$NEG1_OUT"
echo "NEG_SEND_NO_KEY_EC=$NEG1_EC" | tee -a "$REPORT_VARS"

echo "=== negative: cook overwrite refused ==="
NEG2_EC=0
NEG2_OUT=$("$CLI" cook --sponsor-keypair "$DEPLOYER" --out-dir "$COOKED" --rpc-url "$RPC" --cluster localnet 2>&1) || NEG2_EC=$?
echo "$NEG2_OUT"
echo "NEG_COOK_OVERWRITE_EC=$NEG2_EC" | tee -a "$REPORT_VARS"

echo "=== results.env ==="
cat "$REPORT_VARS"
echo "=== realworld done ==="
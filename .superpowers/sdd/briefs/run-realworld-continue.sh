#!/usr/bin/env bash
set -uo pipefail
RPC=http://127.0.0.1:8899
CLI=/workspace-target/release/supersonic-tx
DEP=/workspace/.tmp-operator-sim/deployer.json
COOKED=/workspace/.tmp-operator-sim/cooked
HANDOFF=$(ls -1 "$COOKED"/handoff-*.json | head -1)
TARGET=So11111111111111111111111111111111111111112
RES=/workspace/.tmp-operator-sim/results.env
EPHEM=/workspace/.tmp-operator-sim

echo "=== continue $(date -Iseconds) handoff=$HANDOFF ==="
[ -n "$HANDOFF" ] || { echo "no handoff"; exit 4; }
[ -x "$CLI" ] || { echo "no cli"; exit 2; }

echo "=== simulate ==="
SIM_EC=0
SIM_OUT=$("$CLI" simulate --handoff "$HANDOFF" --target "$TARGET" --amount 100000 --rpc-url "$RPC" --via-router 2>&1) || SIM_EC=$?
echo "$SIM_OUT"
echo "SIMULATE_EC=$SIM_EC" | tee -a "$RES"

echo "=== cast --send ==="
CAST_EC=0
CAST_OUT=$("$CLI" cast --handoff "$HANDOFF" --target "$TARGET" --amount 100000 --rpc-url "$RPC" --via-router --send 2>&1) || CAST_EC=$?
echo "$CAST_OUT" | tee "$EPHEM/cast.out"
echo "CAST_EC=$CAST_EC" | tee -a "$RES"
CAST_SIG=$(echo "$CAST_OUT" | grep -oiE '(signature|sig)[=: ]+[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' || true)
if [ -z "${CAST_SIG:-}" ]; then
  CAST_SIG=$(echo "$CAST_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | head -1 || true)
fi
echo "CAST_SIG=${CAST_SIG:-}" | tee -a "$RES"

echo "=== campaign short --send ==="
CAMP_EC=0
CAMP_OUT=$("$CLI" campaign --handoff "$HANDOFF" --target "$TARGET" --amount 50000 --rpc-url "$RPC" --txs 2 --isolate-intent true --send 2>&1) || CAMP_EC=$?
echo "$CAMP_OUT" | tee "$EPHEM/campaign.out"
echo "CAMPAIGN_EC=$CAMP_EC" | tee -a "$RES"
CAMP_SIGS=$(echo "$CAMP_OUT" | grep -oE '[1-9A-HJ-NP-Za-km-z]{64,88}' | tr '\n' ',' || true)
echo "CAMPAIGN_SIGS=${CAMP_SIGS:-}" | tee -a "$RES"

echo "=== negative: cast --send without keypair ==="
NEG1_EC=0
NEG1_OUT=$("$CLI" cast --target "$TARGET" --amount 100000 --rpc-url "$RPC" --send 2>&1) || NEG1_EC=$?
echo "$NEG1_OUT" | tee "$EPHEM/neg-send.out"
echo "NEG_SEND_NO_KEY_EC=$NEG1_EC" | tee -a "$RES"

echo "=== negative: cook overwrite ==="
NEG2_EC=0
NEG2_OUT=$("$CLI" cook --sponsor-keypair "$DEP" --out-dir "$COOKED" --rpc-url "$RPC" --cluster localnet 2>&1) || NEG2_EC=$?
echo "$NEG2_OUT" | tee "$EPHEM/neg-cook.out"
echo "NEG_COOK_OVERWRITE_EC=$NEG2_EC" | tee -a "$RES"

echo "=== deployer history (for deploy sig) ==="
solana transaction-history "$(solana-keygen pubkey "$DEP")" --url "$RPC" 2>&1 | head -25 | tee "$EPHEM/history.out" || true

echo "=== results.env final ==="
cat "$RES"
echo "=== continue done ==="
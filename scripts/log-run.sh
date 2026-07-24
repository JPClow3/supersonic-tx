#!/usr/bin/env bash
# Append a structured, git-tracked evidence entry to docs/results/RUNS.md from an
# env-style run report (KEY=VALUE lines, e.g. .tmp-operator-sim/phase-b-results.env).
#
# Usage: scripts/log-run.sh <cluster> <path-to-report.env> [note]
#
# Recognized keys (all optional — only present fields are rendered):
#   COMMIT UTC GENESIS PROGRAM_ID DEPLOYER_PUBKEY PROGRAM_OK
#   AIRDROP2_SIG DEPLOY_SIG_CANDIDATE COOK_EC SIMULATE_EC CAST_EC
#   CAST_SIG CAST_BYTES CAST_DECOYS
set -euo pipefail

CLUSTER="${1:?usage: log-run.sh <cluster> <report.env> [note]}"
REPORT="${2:?usage: log-run.sh <cluster> <report.env> [note]}"
NOTE="${3:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$REPO_ROOT/docs/results/RUNS.md"

[ -f "$REPORT" ] || { echo "ERROR: report not found: $REPORT" >&2; exit 1; }

# shellcheck disable=SC1090
set -a; source "$REPORT"; set +a

mkdir -p "$(dirname "$OUT")"
if [ ! -f "$OUT" ]; then
  {
    echo "# Run log"
    echo
    echo "Append-only evidence ledger for localnet/devnet/mainnet runs. Newest entries"
    echo "first. Generated with \`scripts/log-run.sh <cluster> <report.env> [note]\`;"
    echo "each entry pulls straight from the env-style report a smoke/deploy script wrote"
    echo "(see \`.superpowers/sdd/briefs/run-phase-b-smoke.sh\` for the localnet producer)."
    echo
    echo
  } > "$OUT"
fi

ENTRY="$(mktemp)"
{
  echo "## ${UTC:-unknown} — ${CLUSTER} — commit \`${COMMIT:-unknown}\`"
  echo
  [ -n "$NOTE" ] && { echo "$NOTE"; echo; }
  echo "| Field | Value |"
  echo "| --- | --- |"
  [ -n "${GENESIS:-}" ]              && echo "| Genesis | \`$GENESIS\` |"
  [ -n "${PROGRAM_ID:-}" ]           && echo "| Program ID | \`$PROGRAM_ID\` |"
  [ -n "${DEPLOYER_PUBKEY:-}" ]      && echo "| Deployer pubkey | \`$DEPLOYER_PUBKEY\` |"
  [ -n "${PROGRAM_OK:-}" ]           && echo "| Program executable | $([ "$PROGRAM_OK" = 1 ] && echo yes || echo NO) |"
  [ -n "${AIRDROP2_SIG:-}" ]         && echo "| Airdrop sig | \`$AIRDROP2_SIG\` |"
  [ -n "${DEPLOY_SIG_CANDIDATE:-}" ] && echo "| Deploy sig (candidate) | \`$DEPLOY_SIG_CANDIDATE\` |"
  [ -n "${COOK_EC:-}" ]              && echo "| cook exit code | $COOK_EC |"
  [ -n "${SIMULATE_EC:-}" ]          && echo "| simulate exit code | $SIMULATE_EC |"
  [ -n "${CAST_EC:-}" ]              && echo "| cast exit code | $CAST_EC |"
  [ -n "${CAST_SIG:-}" ]             && echo "| Cast signature | \`$CAST_SIG\` |"
  [ -n "${CAST_BYTES:-}" ]           && echo "| Payload size | $CAST_BYTES bytes |"
  [ -n "${CAST_DECOYS:-}" ]          && echo "| Decoys | $CAST_DECOYS |"
  [ -n "${BLOCKER:-}" ]              && echo "| Blocked on | $BLOCKER |"
  echo
} > "$ENTRY"

# Keep newest-first: header is the fixed 7-line preamble written above, everything
# after it is prior entries.
{
  head -n 7 "$OUT"
  cat "$ENTRY"
  tail -n +8 "$OUT"
} > "${OUT}.tmp"
mv "${OUT}.tmp" "$OUT"
rm -f "$ENTRY"

echo "Appended ${CLUSTER} entry (${UTC:-unknown}) to $OUT"

# Smoke test — `supersonic_tx`

Operator checklist after deploy. **Localnet PASS** refreshed 2026-07-24 (commit `971fb96`).
**Devnet PASS** 2026-07-24 (tip `852856d`) — deploy + cook/cast/campaign; see [results/RUNS.md](results/RUNS.md).

Prerequisites:

- Deployed program `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` on target cluster
- Built CLI: `target/release/supersonic-tx`
- Ephemeral deployer/sponsor keypair with SOL (not committed)

## Localnet

Validator: Docker `backpackapp/build:v0.30.1` → `solana-test-validator --reset`
(container `supersonic-localnet`, RPC `http://127.0.0.1:8899`).
Deploy steps: [deploy.md](deploy.md#localnet-validated-path).

Genesis hash changes on every `--reset`. Public explorers (Solscan / explorer.solana.com) will
**not** show localnet signatures — copy-paste sigs below and confirm with
`solana confirm <SIG> --url http://127.0.0.1:8899` while the validator is still running, or re-run
the checklist.

```bash
export RPC=http://127.0.0.1:8899
export DEPLOYER=/path/to/deployer.json   # never commit

solana config set --url "$RPC" --keypair "$DEPLOYER"
solana airdrop 10

solana program deploy target/deploy/supersonic_tx.so \
  --program-id target/deploy/supersonic_tx-keypair.json

supersonic-tx cook --sponsor-keypair "$DEPLOYER" --out-dir /tmp/cooked \
  --rpc-url "$RPC" --cluster localnet

supersonic-tx cast --handoff /tmp/cooked/handoff-*.json \
  --target So11111111111111111111111111111111111111112 --amount 100000 \
  --rpc-url "$RPC" --via-router --send
```

After deploy, an extra airdrop may be required before `cook` (deploy spends rent; first cook can
race funding — see bar-C briefs).

### Expected results (2026-07-24 refresh @ `971fb96`)

| Field | Value |
| --- | --- |
| Genesis (ephemeral) | `Dwas9mCe5QyEPZpJNXewNjhtYpHbcRK2vdN8zjUPfypi` |
| Program ID | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |

| Step | Signature |
| --- | --- |
| Deployer airdrop | `3Rcnw2eq8cp9SgJYEYGhWHqN7SigZvzVyuAtug4T2iYePfzmTSU1U8mwnt6PLy1qxMTJnzeo6g2aLGJs5KsR2kRV` |
| Program deploy | `3ybMFoUh3oVDY51ZBJfj9ZUNjEiduArYbznNRNAm4cvyJFy16opcRkMpnt9QJwnN9dbwDWZV9LnbGfcGrnxgCCF1` |
| Cast `--send` | `39iuxT1gGq2jsRzUwtb5441ME6aebnTuXhSzrd3yfFvttpGW5LBqGKnbsxbu7FvaTJh3ztek5EWjFw7cQqG7tpdE` |

Cast output: simulation OK, **484/1232** bytes, **6** decoys, `solana confirm` → **Finalized**.

Optional dry-run before broadcast:

```bash
supersonic-tx simulate --handoff /tmp/cooked/handoff-*.json \
  --target So11111111111111111111111111111111111111112 --amount 100000 \
  --rpc-url "$RPC" --via-router
```

Reproducible helper (Docker `--network container:supersonic-localnet`,
`RPC=http://127.0.0.1:8899`): `.superpowers/sdd/briefs/run-phase-b-smoke.sh`
(after initial deploy via `run-realworld-sim.sh` or the commands above).

Prior reference run (2026-07-23, same program ID / cast shape): see
`.superpowers/sdd/briefs/bar-c-smoke-2026-07-23.md`.

## Devnet

**PASS** 2026-07-24 on tip `852856d` (deploy + cook/cast/campaign). Ledger:
[results/RUNS.md](results/RUNS.md). Explorer:
https://explorer.solana.com/address/GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9?cluster=devnet

Public faucet may still 429 — use a pre-funded deployer. Reproduce:

```bash
export RPC=https://api.devnet.solana.com
export DEPLOYER=/path/to/funded-deployer.json

supersonic-tx cook --sponsor-keypair "$DEPLOYER" --out-dir /tmp/cooked \
  --rpc-url "$RPC" --cluster devnet

supersonic-tx cast --handoff /tmp/cooked/handoff-*.json \
  --target <TARGET_PUBKEY> --amount 100000 \
  --rpc-url "$RPC" --via-router --send
```

Verify on-chain: `solana program show GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`.
Append new runs with `scripts/log-run.sh` → [results/RUNS.md](results/RUNS.md).

Reference cast (devnet, tip `852856d`):
`2ZnPuuVxhK4oHDNo3vvuFroJsr1n3wVuTuo6Y9xMF9vt76FNojdhg59fh3BVYJwpGSt9xxtaNbMeHZ3xNmz5gRYg`
(313/1232, 3 decoys). Campaign repeatability: two independent cook cycles, all txs Finalized.

## Campaign smoke (optional)

```bash
supersonic-tx campaign --handoff /tmp/cooked/handoff-*.json \
  --target <TARGET_PUBKEY> --amount 100000 \
  --rpc-url "$RPC" --txs 2 --isolate-intent true --send
```

Decoy failures are logged and skipped; real-intent failure exits non-zero.

## Failure modes

| Symptom | Likely cause |
| --- | --- |
| `account not found` on `program show` | Program not deployed on this cluster |
| `Underfunded` / debit prior credit | Cook before post-deploy airdrop settles |
| `RPC genesis hash does not match` | `--cluster` / handoff cluster ≠ RPC URL |
| `RPC blockhash not found` / transport errors on `cast` | Transient RPC; CLI rebuilds once with a fresh blockhash (`is_transient_rpc`) |
| `RPC insufficient funds for fee` | Fee payer underfunded — not retried as transient |
| Router error | Program missing or not executable; deploy first |
| Faucet 429 | Use localnet or a pre-funded devnet wallet |

Source briefs: `.superpowers/sdd/briefs/bar-c-smoke-2026-07-23.md`,
`.superpowers/sdd/briefs/phase-b-live-proof-2026-07-24.md`.

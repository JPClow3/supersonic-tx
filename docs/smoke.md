# Smoke test — `supersonic_tx`

Operator checklist after deploy. **Localnet PASS** recorded 2026-07-23; **devnet not run**
(funded deployer unavailable; public faucet HTTP 429).

Prerequisites:

- Deployed program `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` on target cluster
- Built CLI: `target/release/supersonic-tx`
- Ephemeral deployer/sponsor keypair with SOL (not committed)

## Localnet

Validator: Docker `backpackapp/build:v0.30.1`, RPC `http://127.0.0.1:8899`.
Deploy steps: [deploy.md](deploy.md#localnet-validated-path).

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

### Expected results (2026-07-23 reference run)

| Step | Signature |
| --- | --- |
| Deployer airdrop | `2kP7geJdGFzBTrtRoLLCMgyJjrjh2zhBE4XrXr2p7m6VhCyPLGe6vJWj9L6SX9pH7B8E9CGcA7EbDhH4kbPJFDs3` |
| Program deploy | `2GfaPcBaWsJNvQtjtgP72aEBnK55qJ2znQSRd59ok781q8FrENf7njFGx4ZghuSAii7arHAG8R6EAsMiL4NFdfyT` |
| Cast `--send` | `3bx8PvSJBCqksKurDXGkhepumDotj5DfXj68XZeLuxL9ottieDJqGn2DDZoCa1WcjMh8wwSZsSfm9mMWKGFYLW7s` |

Cast output: simulation OK, **484/1232** bytes, **6** decoys.

Optional dry-run before broadcast:

```bash
supersonic-tx simulate --handoff /tmp/cooked/handoff-*.json \
  --target So11111111111111111111111111111111111111112 --amount 100000 \
  --rpc-url "$RPC" --via-router
```

## Devnet

**Blocked without funded deployer.** Do not assume `solana airdrop` succeeds on public devnet.

When funded:

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
| `Underfunded` | Cook with insufficient `--fee-payer-lamports` or spent balance |
| `RPC genesis hash does not match` | `--cluster` / handoff cluster ≠ RPC URL |
| Router error | Program missing or not executable; deploy first |
| Faucet 429 | Use localnet or a pre-funded devnet wallet |

Source brief: `.superpowers/sdd/briefs/bar-c-smoke-2026-07-23.md`.

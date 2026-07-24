# Bar C localnet smoke - 2026-07-23

**Status: PASS (localnet)** — devnet not run (faucet 429; program not on devnet).

## Environment

- Validator: Docker `backpackapp/build:v0.30.1` → `solana-test-validator --reset` (container `supersonic-localnet`, RPC `http://127.0.0.1:8899`)
- Program ID: `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` (matches keypair + `declare_id!`)
- CLI: `target/release/supersonic-tx` built in Docker `rust:latest` (`cargo build --release -p supersonic-tx-cli --locked`)

## Commands

```bash
# deploy (inside Docker, host network)
solana config set --url http://127.0.0.1:8899 --keypair <DEPLOYER.json>
solana airdrop 10
solana program deploy target/deploy/supersonic_tx.so --program-id target/deploy/supersonic_tx-keypair.json

supersonic-tx cook --sponsor-keypair <DEPLOYER.json> --out-dir /tmp/cooked \
  --rpc-url http://127.0.0.1:8899 --cluster localnet

supersonic-tx cast --handoff /tmp/cooked/handoff-<ts>.json \
  --target So11111111111111111111111111111111111111112 --amount 100000 \
  --rpc-url http://127.0.0.1:8899 --via-router --send
```

## Signatures

| Step | Signature |
| --- | --- |
| Deployer airdrop | `2kP7geJdGFzBTrtRoLLCMgyJjrjh2zhBE4XrXr2p7m6VhCyPLGe6vJWj9L6SX9pH7B8E9CGcA7EbDhH4kbPJFDs3` |
| Program deploy | `2GfaPcBaWsJNvQtjtgP72aEBnK55qJ2znQSRd59ok781q8FrENf7njFGx4ZghuSAii7arHAG8R6EAsMiL4NFdfyT` |
| Cook / fund | (multiple transfers; see deployer history on localnet) |
| Cast `--send` | `3bx8PvSJBCqksKurDXGkhepumDotj5DfXj68XZeLuxL9ottieDJqGn2DDZoCa1WcjMh8wwSZsSfm9mMWKGFYLW7s` |

Cast result: simulation OK, **484/1232** bytes, **6** decoys.

## Devnet caveat

Public devnet faucet rate-limited (HTTP 429). No devnet deploy or smoke at this time.

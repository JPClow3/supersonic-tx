# Phase B live-proof — localnet refresh 2026-07-24

**Status: PASS (localnet)** — devnet not run (no funded deployer; faucet 429 risk unchanged).

## Environment

| Field | Value |
| --- | --- |
| Branch | `feature/bar-c` |
| Commit at smoke | `971fb96` |
| Tag nearby | `v0.1.0-bar-c` (tip is 2 commits ahead at smoke time) |
| Program ID | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |
| Cluster | localnet (Docker `supersonic-localnet`, RPC `http://127.0.0.1:8899`) |
| Genesis hash | `Dwas9mCe5QyEPZpJNXewNjhtYpHbcRK2vdN8zjUPfypi` (ephemeral after `--reset`) |
| CLI | `/workspace-target/release/supersonic-tx` (Docker volume) |
| Image | `backpackapp/build:v0.30.1` with `--network container:supersonic-localnet` |

## Flow

1. `docker rm -f supersonic-localnet` + fresh `solana-test-validator --reset`
2. `run-realworld-sim.sh` → airdrop + program deploy (cook raced funding → exit 4)
3. Extra airdrop + `run-phase-b-smoke.sh` → cook → simulate → `cast --via-router --send`

Ephemeral deployer under `.tmp-operator-sim/` (**gitignored**; not committed).

## Signatures

| Step | Signature | Confirm |
| --- | --- | --- |
| Deployer airdrop | `3Rcnw2eq8cp9SgJYEYGhWHqN7SigZvzVyuAtug4T2iYePfzmTSU1U8mwnt6PLy1qxMTJnzeo6g2aLGJs5KsR2kRV` | Finalized |
| Program deploy (ProgramData hx) | `3ybMFoUh3oVDY51ZBJfj9ZUNjEiduArYbznNRNAm4cvyJFy16opcRkMpnt9QJwnN9dbwDWZV9LnbGfcGrnxgCCF1` | Finalized |
| Cast `--via-router --send` | `39iuxT1gGq2jsRzUwtb5441ME6aebnTuXhSzrd3yfFvttpGW5LBqGKnbsxbu7FvaTJh3ztek5EWjFw7cQqG7tpdE` | Finalized |

Cast shape: **484/1232** bytes, **6** decoys.

## Explorer note

Localnet signatures are **not** on public explorers. Judges should copy-paste sigs from README
**Deployments** / this brief, or re-run Docker localnet smoke. Devnet would unlock Solscan links
once a funded wallet exists.

## Devnet blocker

No keypairs in repo. Public faucet may HTTP 429. Path remains: fund a deployer off-repo →
`docs/deploy.md` → `docs/smoke.md` → paste Solscan URLs into README.

## Secrets

No keypairs, sponsor secrets, or cooked keys committed.

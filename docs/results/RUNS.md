# Run log

Append-only evidence ledger for localnet/devnet/mainnet runs. Newest entries
first. Generated with `scripts/log-run.sh <cluster> <report.env> [note]`;
each entry pulls straight from the env-style report a smoke/deploy script wrote
(see `.superpowers/sdd/briefs/run-phase-b-smoke.sh` for the localnet producer).

## 2026-07-24T14:04:10Z — devnet — commit `852856dbecb616287a91e39d8b4ab5cd3d6f6f0d`

Repeatability run 2/2: independent cook cycle (fresh decoy sinks) + campaign --txs 3 --isolate-intent true --send. All 4 txs confirmed Finalized on devnet. Decoy sigs: `2SvLqQTkMLxEAjztAGuhCwS3NckD4tsDuQdGiGoK6tZcFtvuobEHetBLECpeegnHSp4WVjZWvK6dTNFv78YLnxJK` (379/1232), `4CSE8si3Fkfxzk6zMntooH32tCqCpXAzD9G13w8XzidNtDFupaagaonxBdh4P6Uk48ALhd1EAAMpjsYdXQwfDxaR` (343/1232), `GKqP76Ppg3JM2ryLAi4duzE59uciiow1QDaa3J4CQaP8BEm1nm6ZXCCbzmEwwaNnTuPdMbWfd1Qi3fcF4SEBnz4` (375/1232). Same commit, same program, same deployer, independent decoy accounts — output shape (byte sizes, decoy counts, exit codes) is consistent across both runs.

| Field | Value |
| --- | --- |
| Program ID | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |
| cast exit code | 0 |
| Cast signature | `2ZnPuuVxhK4oHDNo3vvuFroJsr1n3wVuTuo6Y9xMF9vt76FNojdhg59fh3BVYJwpGSt9xxtaNbMeHZ3xNmz5gRYg` |
| Payload size | 313/1232 bytes |
| Decoys | 3 |

## 2026-07-24T14:01:50Z — devnet — commit `852856dbecb616287a91e39d8b4ab5cd3d6f6f0d`

Repeatability run 1/2: cook + campaign --txs 3 --isolate-intent true --send. All 4 txs (3 decoy-only + 1 real-intent) broadcast and confirmed Finalized on public devnet — unlike localnet these resolve on Solscan/Explorer. Decoy sigs: `4SR3qeLZs9faJdEuor7ykmFVJNr336iLrF1BvbSBcJfajbmmgkXBCTYoH16jH4u2pMuvoB8EPfSv93rZMTsEJfeT` (379/1232), `59ae2qVRSbreP7NXADFq7isZVisLjfME7ZzUxKVMJMrenG96QyctzYeZ4SDYX2KUAJbLxVEEVu2L95neKdYvRGLm` (381/1232), `5cmUcXcPrLoZMnkNJGUkQVt1xVv7jrD28E8XszExqXVLhUyQcWpLUF1ryLeNgMNnWxH57VBtXiBJZDoAG6gV3Wk1` (347/1232).

| Field | Value |
| --- | --- |
| Program ID | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |
| cast exit code | 0 |
| Cast signature | `3SpTt9ZLpprm1mqTQtWf7cxEtTzGdfgzfXL59sSNaeSXyui62UAXFU3rBgXx1xG9yyPCqmsfFoXJGm7sLHxF5V4F` |
| Payload size | 309/1232 bytes |
| Decoys | 3 |

## 2026-07-24T14:00:00Z — devnet — commit `852856dbecb616287a91e39d8b4ab5cd3d6f6f0d`

Real devnet deploy. Fixed a bug on the way: DEVNET_GENESIS_HASH / MAINNET_GENESIS_HASH in crates/supersonic-tx-cli/src/main.rs were truncated to 32 chars instead of the real 44-char genesis hashes, so cook/campaign rejected every real devnet/mainnet RPC outright regardless of funding. Corrected both constants and rebuilt the CLI before this run. Program executable and owned by the upgradeable BPF loader, confirmed via `solana program show`; explorer: https://explorer.solana.com/address/GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9?cluster=devnet

| Field | Value |
| --- | --- |
| Program ID | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |
| Deployer pubkey | `A21K9UPZeB8MD4Zvp8EoKa77JAebrbgagjMMMWEfXKXA` |
| Program executable | yes |

## 2026-07-24T13:27:50Z — devnet — commit `852856dbecb616287a91e39d8b4ab5cd3d6f6f0d`

Deploy blocked: deployer wallet above needs devnet SOL before program deploy / cook / cast can run (see docs/deploy.md#devnet). Fund it via https://faucet.solana.com (manual, requires a captcha) or a transfer from another funded devnet wallet, then re-run the devnet steps in docs/deploy.md and docs/smoke.md and log the result with this same script.

| Field | Value |
| --- | --- |
| Program ID | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |
| Deployer pubkey | `A21K9UPZeB8MD4Zvp8EoKa77JAebrbgagjMMMWEfXKXA` |
| Blocked on | public devnet faucet rate-limited on two consecutive solana airdrop attempts (2 SOL, then 1 SOL) against https://api.devnet.solana.com |

## 2026-07-24T02:12:32Z — localnet — commit `971fb9641f8139a8dffaa2f28cf53227d34bad80`

Localnet genesis is ephemeral (regenerated on every validator --reset) — public explorers will not resolve this signature. Confirm with `solana confirm <SIG> --url http://127.0.0.1:8899` while that validator instance is still running, or re-run `.superpowers/sdd/briefs/run-phase-b-smoke.sh`.

| Field | Value |
| --- | --- |
| Genesis | `Dwas9mCe5QyEPZpJNXewNjhtYpHbcRK2vdN8zjUPfypi` |
| Program ID | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |
| Deployer pubkey | `A21K9UPZeB8MD4Zvp8EoKa77JAebrbgagjMMMWEfXKXA` |
| Program executable | yes |
| Airdrop sig | `UfHLXoUv1A5BGobnmWRP2MW9SQAb1dLKa4rFje6PYe1myFqX16F4ytHqBpbHnzPfQ36NZohQKW6VGQpK9Da6TzZ` |
| cook exit code | 0 |
| simulate exit code | 0 |
| cast exit code | 0 |
| Cast signature | `39iuxT1gGq2jsRzUwtb5441ME6aebnTuXhSzrd3yfFvttpGW5LBqGKnbsxbu7FvaTJh3ztek5EWjFw7cQqG7tpdE` |
| Payload size | 484/1232 bytes |



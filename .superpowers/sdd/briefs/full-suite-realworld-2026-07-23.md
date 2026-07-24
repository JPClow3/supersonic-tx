# Full suite + real-world localnet - 2026-07-23

**Status: PASS** (automated suite + operator simulation on fresh localnet)

| Field | Value |
| --- | --- |
| Branch | `feature/bar-c` |
| Commit | `00d99d3` |
| Program ID | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |
| Cluster | localnet (Docker `supersonic-localnet`, RPC `http://127.0.0.1:8899`) |
| Genesis hash | `Gc3dVto2s8sVjuAoSoZLdQ2LWnCeJWJwoMna6cWJhjht` (ephemeral after `--reset`) |
| Suite log | `.superpowers/sdd/briefs/full-suite-2026-07-23.log` |

## Part 1 - Automated tests (Docker `rust:latest`)

| Step | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS (`FMT_EXIT=0`) |
| `cargo test --workspace --locked` | PASS (`WORKSPACE_TEST_EXIT=0`) - **50** tests |
| `programs/supersonic-tx-tests` `--locked` | PASS (`ROUTER_TEST_EXIT=0`) - **6** tests |
| **Total** | **56** passed / 0 failed |

Summary line in log: `SUMMARY fmt=0 workspace=0 router=0`.

## Part 2 - Real-world localnet simulation

Fresh validator: `docker rm -f supersonic-localnet` then `solana-test-validator --reset`. Operator steps ran in `backpackapp/build:v0.30.1` with `--network container:supersonic-localnet` so CLI `--cluster localnet` accepts `http://127.0.0.1:8899`. CLI binary: Docker `cargo build --release -p supersonic-tx-cli --locked`. Ephemeral deployer under `.tmp-operator-sim/` (gitignored; **not committed**).

| Scenario | Result | Notes / signatures |
| --- | --- | --- |
| Deployer airdrop | PASS | `HHi2QLLFHKNuR6XBRvHR2oLoj1FWiLArUKoYp8mb9UTiDndGNnC62yZ7u8fMhuJfMQY4tfccCbhmsbdcUNNuWzt` (+ follow-up `uMHu5r7Sm1bsnvPUyZQCn7h2xjqXs4EkH5J43epJi6KNorbtugA9MHrHkZLFR2ncNox1e7J3g4yUUZ7wn1BGkDH`) |
| Program deploy | PASS | Executable; sig `4Q2kKrDPDiUzaioDxtFDifEH5eZey9Cfr2swNNZfj26PXUJce93AmntfZgHSPDHPrrYPmMfF3vgbUPuBDR5pDn2k` (slot 59) |
| `cook` | PASS | Handoff `handoff-1784858106.json` (first cook raced post-deploy funding; retry after extra airdrop OK) |
| `simulate --via-router` | PASS | 480/1232 bytes, 6 decoys |
| `cast --via-router --send` | PASS | **484/1232**, 6 decoys; sig `3K2RkHy1LK3RpbKbNyDcn3z2YKkvnMLZazw8tfrPvKhEUvL9e29p9MR9VBbW3r8BRH5yJjYDwxwsXqfdWcdj7sCt` (Finalized) |
| `campaign --txs 2 --isolate-intent true --send` | PASS | DecoyOnly `mGp3FHCDGoAUMmn9zf7cuWCgnf8GqmMUGs1KFzuuwLeXjNP45BmDLEG4GmjEKxCHAorahTWuLjHXR9BEDPSsZgK`; DecoyOnly `4whxb3smwtHkSBYfCjQ2agWYTnWJorZnr5FUF6DxYj1r3YEtNHRhC7qWvJoNxB1xY5tHi1UNSjN73hUeahuHxtFT`; RealIntent `JXqWgaVozToHFaVAkL7MU3iy9SAuGYCjmmaixRrohQgh26u65sPRKpnBgm3hjjKds5885mQXARmrVamtFusGVvU` |
| Negative: `cast --send` without keypair/handoff | PASS (exit 1) | `provide exactly one of --keypair or --handoff` |
| Negative: `cook` overwrite existing keys | PASS (exit 1) | `KeyFileExists(.../keys/fee_payer.json)` |

## Failures / caveats

- None blocking. Transient: first `cook` after deploy returned RPC debit error until extra airdrop/confirmation; re-run succeeded.
- Operator helpers used `--network container:supersonic-localnet` because `host.docker.internal` fails the localnet genesis URL check (requires `127.0.0.1`/`localhost` in RPC URL).
- Devnet not exercised (funded deployer / faucet constraint unchanged).

## Secrets

No keypairs, sponsor secrets, or cooked keys committed. `.tmp-operator-sim/` added to `.gitignore`.
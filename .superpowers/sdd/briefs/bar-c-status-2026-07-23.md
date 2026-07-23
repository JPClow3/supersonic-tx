# Bar C status - 2026-07-23 (finish pass)

Branch: `feature/bar-c`

## Workspace

Root members: `programs/supersonic-tx`, `crates/supersonic-tx-core`, `crates/supersonic-tx-sdk`, `crates/supersonic-tx-cli`, `crates/account-cooker`. Router ProgramTest: `programs/supersonic-tx-tests/` (standalone workspace + `Cargo.lock`, excluded from root). `[profile.release] overflow-checks = true`.

## Rust gates (re-run 2026-07-23)

Docker `rust:latest` with `supersonic-cargo-registry`, `supersonic-cargo-git`, `supersonic-target` volumes:

- `cargo fmt --all -- --check` — exit 0
- `cargo test --workspace --locked` — exit 0 (**50** unit tests)
- `cargo test --locked --manifest-path programs/supersonic-tx-tests/Cargo.toml` — exit 0 (**6** tests)

**Total: 56** executable tests. Log: `.superpowers/sdd/briefs/bar-c-cargo-test-2026-07-23.log`.

## Anchor / SBF (Task 20)

**Artifact:** `target/deploy/supersonic_tx.so` (**196640** bytes).

**Tooling:** `backpackapp/build:v0.30.1`, `.superpowers/sdd/briefs/anchor-build-docker.sh`, `.superpowers/sdd/briefs/pin-sbf-lock.sh`.

**Full `anchor build`:** still **exit 1** on host Cargo 1.79 `cargo metadata` vs crates.io edition2024 manifests; SBF `.so` from earlier compile retained. IDL optional — not produced in this pass.

## Blocked

- Devnet deploy + cook/simulate/cast smoke (no operator keys in repo).
- Green GitHub Actions run not stored in repo.

See `.superpowers/sdd/briefs/bar-c-finish-report-2026-07-23.md`.

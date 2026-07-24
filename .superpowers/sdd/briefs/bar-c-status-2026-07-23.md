# Bar C status - 2026-07-23 (finish pass)

Branch: `feature/bar-c`

## Workspace members (root)

- `programs/supersonic-tx`
- `crates/account-cooker`
- `crates/supersonic-tx-core`
- `crates/supersonic-tx-sdk`
- `crates/supersonic-tx-cli`

Excluded: `programs/supersonic-tx-tests/` (standalone workspace + `Cargo.lock`). `[profile.release] overflow-checks = true`.

## Lock policy (dual)

| Use | Lock | Toolchain |
|-----|------|-----------|
| Native / CI `cargo test --workspace --locked` | Root `Cargo.lock` (full members) | Docker `rust:latest` |
| SBF / `cargo-build-sbf` | `.superpowers/sdd/briefs/Cargo.lock.sbf.v3` (slim program+core pins from `7396e3b`) | `backpackapp/build:v0.30.1` |

Refresh SBF snapshot: slim members, `pin-sbf-lock.sh` in backpack, copy to `Cargo.lock.sbf.v3`, restore members, `cargo generate-lockfile` on `rust:latest`. Full workspace metadata on SBF Cargo pulls edition2024 crates and fails.

## Rust gates (re-run 2026-07-23)

Docker `rust:latest` (volumes: `supersonic-cargo-registry`, `supersonic-cargo-git`, `supersonic-target`):

- `cargo fmt --all -- --check` — exit 0
- `cargo test --workspace --locked` — exit 0 (**50** unit tests)
- `cargo test --locked --manifest-path programs/supersonic-tx-tests/Cargo.toml` — exit 0 (**6** tests)

**Total: 56** tests. Log: `.superpowers/sdd/briefs/bar-c-cargo-test-2026-07-23.log`.

## Anchor / SBF (Task 20)

**Artifact:** `target/deploy/supersonic_tx.so` (**196640** bytes, preserved).

**Tooling:** `backpackapp/build:v0.30.1`, `anchor-build-docker.sh`, `pin-sbf-lock.sh`.

**Rebuild with full workspace lock:** fails (edition2024 / Cargo 1.79). Use SBF lock snapshot or slim workspace for new `.so`.

**Full `anchor build`:** still exit 1 on host metadata; prior `.so` retained.

## Blocked

- Devnet deploy + smoke (no keys in repo).
- Green GitHub Actions artifact not in repo.

See `bar-c-finish-report-2026-07-23.md`.

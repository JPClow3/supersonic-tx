# Bar C finish report — 2026-07-23

Branch: `feature/bar-c`

## DONE

| Goal | Result |
| --- | --- |
| Restore workspace members | Root includes program, core, sdk, cli, account-cooker; `programs/supersonic-tx-tests` excluded with own `[workspace]` + `Cargo.lock`. |
| SBF artifact | `target/deploy/supersonic_tx.so` present (**196640** bytes, 2026-07-23). |
| Linux Rust gates | Docker `rust:latest` + volumes: `cargo fmt --check` OK; `cargo test --workspace --locked` **50** tests OK; `cargo test --locked --manifest-path programs/supersonic-tx-tests/Cargo.toml` **6** router tests OK (**56** total). Log: `.superpowers/sdd/briefs/bar-c-cargo-test-2026-07-23.log`. |
| Workspace lock | Regenerated root `Cargo.lock` (v4) aligned with full members + `[profile.release] overflow-checks = true`. |
| Router test crate fix | Added `anchor-lang` + `solana-program` dev-deps to `programs/supersonic-tx-tests/Cargo.toml`. |
| Status docs | Updated `bar-c-status-2026-07-23.md`, `progress.md`, this report. |

## Still blocked / partial

| Item | Status |
| --- | --- |
| Full green `anchor build` | **PARTIAL** — `backpackapp/build:v0.30.1`: host `cargo metadata` fails on crates.io **edition2024** (`clap_lex` 1.1.0); `pin-sbf-lock.sh` needs version-qualified pins. SBF `.so` from prior successful compile remains on disk; IDL/verify tail not green. Log: `.superpowers/sdd/briefs/bar-c-anchor-build-2026-07-23.log`. |
| Devnet deploy + smoke | **BLOCKED** — no RPC operator keypair in repo (by design); README deployments still empty. Follow `docs/deploy.md` when keys available. |
| GitHub Actions artifact | **Not in repo** — local Docker evidence only. |
| Task 20 / I12 | **PARTIAL** — Rust runtime gates green; Anchor IDL gate + deploy smoke open. |

## Commands (evidence)

```bash
docker run --rm -v "$PWD:/workspace" -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target rust:latest \
  bash -c 'rustup component add rustfmt && cargo fmt --all -- --check && cargo test --workspace --locked'

docker run --rm ... -w /workspace/programs/supersonic-tx-tests -e CARGO_TARGET_DIR=/workspace-target/program-tests \
  rust:latest cargo test --locked
```

## Dual-lock note

- **App/CI:** root `Cargo.lock` v4 + `rust:latest` (full workspace).
- **SBF/Anchor 0.30.1:** requires pinned transitive deps or separate lock policy; helper scripts under `.superpowers/sdd/briefs/` (`pin-sbf-lock.sh`, `anchor-build-docker.sh`).

## Artifact paths

- `target/deploy/supersonic_tx.so`
- `.superpowers/sdd/briefs/bar-c-cargo-test-2026-07-23.log`
- `.superpowers/sdd/briefs/bar-c-anchor-build-2026-07-23.log`

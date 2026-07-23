# Bar C status - 2026-07-23 (updated)

Branch: `feature/bar-c`

## Anchor / SBF (Task 20)

**Artifact:** `target/deploy/supersonic_tx.so` (196640 bytes) built via `backpackapp/build:v0.30.1` after lock v3 + transitive pins (blake3 1.5.5, borsh 1.5.5, cc/jobserver, indexmap 2.6.0, unicode-segmentation 1.11.0, zeroize_derive 1.4.2, proc-macro-crate 3.1.0, etc.).

**Tooling:** Docker `backpackapp/build:v0.30.1` (Anchor 0.30.1 / Solana 1.18 SBF). Helper: `.superpowers/sdd/briefs/anchor-build-docker.sh`.

**Workspace:** Root members slimmed to `programs/supersonic-tx` + `crates/supersonic-tx-core` for SBF-compatible `cargo metadata`. `[profile.release] overflow-checks = true`. Program `idl-build` feature added. Router ProgramTest moved to standalone `programs/supersonic-tx-tests/` (own `[workspace]`, excluded from root).

**Remaining:** Full `anchor build` (IDL/verify tail) still exits 1 on `edition2024` metadata when Anchor runs host Cargo 1.79 against crates.io edge; SBF `.so` step completes. Re-expand CLI/SDK/cooker workspace members only with matching lock policy or separate app lock. No deploy/smoke.

## Rust gates

Prior Linux `cargo test --workspace --locked` (56 tests) used full workspace + lock v4; re-run needed after workspace/lock change (`rust:latest` + `--manifest-path` for moved router tests).

## Windows

Host: `cargo` 1.97; no native Anchor/Solana. Use Docker for Anchor/SBF.

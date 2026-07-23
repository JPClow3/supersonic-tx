# Tasks 18 and 20 verification report

Date: 2026-07-23
Branch: `feature/bar-c`
Environment: Docker Desktop 4.83.0, Linux x86_64 engine 29.6.2, `rust:latest` (Rust 1.97.1). WSL2 had only the stopped `docker-desktop` distribution. The committed `Cargo.lock` was present, so all Cargo gates used `--locked`.

## Final evidence

1. Linux formatting gate
   - Command: `docker run --rm -v "${PWD}:/workspace" -w /workspace rust:latest bash -c 'rustup component add rustfmt && cargo fmt --all -- --check'`
   - Exit: `0`
   - Excerpt: `info: downloading component rustfmt` followed by clean exit.

2. Linux workspace test gate
   - Command: `docker run --rm -v "${PWD}:/workspace" -v supersonic-cargo-registry:/usr/local/cargo/registry -v supersonic-cargo-git:/usr/local/cargo/git -v supersonic-target:/workspace-target -w /workspace -e CARGO_TARGET_DIR=/workspace-target rust:latest cargo test --workspace --locked`
   - Final exit: `0`
   - Excerpts: `18 passed`, `4 passed`, router integration `6 passed`, CLI `6 passed`, core `1 passed`, SDK `21 passed`; all doc-tests passed (56 executable tests total).
   - Non-blocking warnings: Anchor/Solana 1.18 macro `unexpected cfg` warnings, one dead-code warning, and Solana 1.18 future-incompatibility notice.

3. Anchor availability and fallback
   - Command: `docker run --rm -v "${PWD}:/workspace" -w /workspace rust:latest bash -c 'anchor --version'`
   - Exit: `127`
   - Excerpt: `anchor: command not found`.
   - Fallback: same cached Linux container setup, then `cargo build --locked -p supersonic-tx`.
   - Exit: `0`
   - Excerpt: `Finished dev profile`.
   - This verifies the native Rust program build, not an Anchor/SBF artifact.

## Failures found and fixed

Early Linux runs found Solana 1.18 fee/signer API errors, stale CLI/campaign integration, ProgramTest/Anchor lifetime and simulation API incompatibilities, a non-rent-exempt CPI recipient, and a bounded Benford sampler that biased leading digit 1. Focused fixes landed in `8c8bbe3`, `9e4099a`, `b3fe184`, `bd64492`, and `229f93f`. The Benford test also passed five consecutive runs before the final full green suite.

## Status

- Task 18: **DONE (qualified)**. Linux Rust CI-equivalent formatting and locked workspace tests are green, and the native program fallback build is green. The separate Anchor/SBF job was not reproduced because Anchor/Solana CLI tooling is absent.
- Task 20: **PARTIAL / BLOCKED**. Linux native verification is green and removes the Windows WDAC blocker. Remaining release evidence is a real `anchor build`/SBF artifact plus an explicitly targeted local/devnet deployment smoke. No deployment or real-SOL claim is made here.
## Task 20 follow-up — Anchor 0.30.1 / SBF gate (2026-07-23 evening)

Environment: Windows host has no native `cargo`; WSL `bash` unavailable (`execvpe(bash) failed`). Reused **Docker Desktop** Linux engine (same as Task 18).

### Tooling install (AVM path)

1. `rust:latest` container + Solana CLI **1.18.26** (Anza installer) + **AVM** + `avm install 0.30.1` / `avm use 0.30.1`.
   - Result: `anchor-cli 0.30.1`, `solana-cli 1.18.26`.
   - Helper script: `.superpowers/sdd/briefs/anchor-build-docker.sh` (volumes: `supersonic-avm`, `supersonic-solana`).

2. Official image: `backpackapp/build:v0.30.1` (`anchor-cli 0.30.1`, `solana-cli 1.18.17`, `cargo 1.79.0`).

### `anchor build` attempts

| Step | Command / context | Exit | Outcome |
|------|-------------------|------|---------|
| A | AVM + `rust:latest`, committed `Cargo.lock` **v4** | 1 | Anchor requires `[profile.release] overflow-checks = true` → added to workspace `Cargo.toml`. |
| B | After overflow-checks, AVM + `rust:latest`, lock **v4** | 1 | `cargo-build-sbf` (Cargo **1.75**) cannot parse lock v4. |
| C | Lock header manually set to **v3** (reverted afterward) | 1 | Shared registry pulled **edition2024** manifests (`crypto-common 0.2.2`, `indexmap 2.14.0`). |
| D | `backpackapp/build:v0.30.1`, committed lock **v4** | 1 | `cargo metadata` fails on **edition2024** crate manifests (e.g. `block-buffer 0.12.1`). |
| E | Fresh `cargo generate-lockfile` inside backpack (Cargo 1.79) then `anchor build` | 1 | Resolver still fetched **edition2024** crates (`clap_lex 1.1.0`) from current crates.io index. |
| F | `cargo-build-sbf --manifest-path programs/supersonic-tx/Cargo.toml` (backpack) | 1 | Same edition2024 metadata failure. |

**No SBF artifact produced.** `target/deploy/` contains only `supersonic_tx-keypair.json` (no `supersonic_tx.so`). README **Deployments** still: *None recorded* — no on-chain deploy attempted.

### Root blocker (honest)

Committed **`Cargo.lock` (v4, Rust 1.97 resolver)** and the **2026 crates.io index** pull dependencies whose manifests require **edition2024**, which **Anchor 0.30.1 / Solana 1.18 SBF Cargo (1.75–1.79)** cannot parse. Native Linux `cargo test --workspace --locked` remains green on `rust:latest`; the **I12 Anchor/SBF gate** is not closed until toolchain/lock alignment (e.g. dual-lock policy, pinned older transitive deps, or upgraded Anchor/Solana build image) yields a real `target/deploy/supersonic_tx.so`.

### Task 20 status (unchanged)

**PARTIAL / BLOCKED** — tooling installed and documented; **no** successful `anchor build`; **no** deployment smoke.

## Bar C finish re-run — 2026-07-23 (Docker)

After restoring full workspace members and regenerating `Cargo.lock`:

- `cargo fmt --all -- --check` — exit 0
- `cargo test --workspace --locked` — exit 0 (50 tests)
- `cargo test --locked --manifest-path programs/supersonic-tx-tests/Cargo.toml` — exit 0 (6 tests)

Machine-readable log: `.superpowers/sdd/briefs/bar-c-cargo-test-2026-07-23.log`.

Task 18 remains **DONE (qualified)** with reproducible log on this host. Task 20 remains **PARTIAL** (Anchor IDL/metadata + deploy smoke).

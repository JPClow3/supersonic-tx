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
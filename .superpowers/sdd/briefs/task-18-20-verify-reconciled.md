# Task 18 / 20 / I12 — reconciled verification (2026-07-23)

**Branch:** `feature/bar-c`  
**Git HEAD:** `59abc37`  
**Authoritative green narrative (git):** `59abc37:.superpowers/sdd/briefs/task-18-20-verify-report.md`  
**This document:** resolves conflicting agent claims using filesystem + git only.

## Timeline (git)

| Commit | What changed |
| --- | --- |
| `229f93f` | Code fix: bounded Benford sampler in `crates/supersonic-tx-sdk/src/noise.rs` (consistent with fix before green suite in the verify report). |
| `59abc37` | **Added** `task-18-20-verify-report.md` claiming Docker Linux `cargo fmt --check`, `cargo test --workspace --locked` exit 0 (56 executable tests), and `cargo build --locked -p supersonic-tx` fallback; Anchor absent (exit 127). |
| Working tree (uncommitted) | A later session **overwrote** the report on disk with an OOM failure (`rust:bookworm`, no cargo/target volumes) and falsely stated the report was Missing before this run. **Restored** report content from `59abc37` for the canonical path. |

## What is proven

1. Commits `229f93f` and `59abc37` exist on current HEAD; `59abc37` is the tip.
2. The repository **contains a committed verification report** describing a green Linux Docker run using `rust:latest` with named volumes and `CARGO_TARGET_DIR=/workspace-target`.
3. Prior fix commits cited in that report exist in history and align with failures described before the claimed green run.
4. `.github/workflows/ci.yml` exists (from `46d1a17`) but **no green GitHub Actions log or artifact** is stored in this repo.
5. Windows host still has WDAC / no usable WSL userland (consistent with both reports).

## What is not proven

1. **No machine-readable test log** with test result ok or 56 passed anywhere in the workspace; only prose in `59abc37` report.
2. **Re-run on this host (later session) failed** OOM compiling libc in `rust:bookworm` without the volume setup in `59abc37` — does not disprove the earlier setup but shows **current host cannot reproduce without matching Docker resources/volumes**.
3. **Anchor build, deploy, and smoke** — not evidenced in repo.
4. **Remote Linux CI** — not evidenced in repo.

## Reconciliation of agent claims

| Claim | Verdict |
| --- | --- |
| Agent A: Task 18 DONE, 56 tests, commits `229f93f`/`59abc37` | **Partially supported:** commits and committed report support **qualified** Task 18; not supported as independently reproducible runtime proof without saved log or matching Docker re-run. |
| Agent B: I12 BLOCKED, OOM, no prior report | **Partially correct:** OOM re-run is real; **incorrect** that no prior report existed — `59abc37` added it before the overwrite. Full I12 remains **incomplete**. |

## Status (evidence-only)

| Item | Status |
| --- | --- |
| **Task 18** (CI + lockfile / Linux Rust gates) | **DONE (qualified)** — Linux fmt + locked workspace tests + native program fallback **documented green in git** (`59abc37`); re-run with same Docker volumes or green GitHub Actions recommended. |
| **Task 20** (Bar C / full release verification) | **PARTIAL / BLOCKED** — Anchor/SBF, deployment, and smoke still missing. |
| **I12** (runtime verification umbrella) | **PARTIAL** — Linux Rust subset in committed report; remote CI, Anchor, deploy, smoke, and reproducible log on this host **not satisfied**. |

## Note on commits vs. missing logs

Fixes in `229f93f` and related commits are **consistent with** a green suite before `59abc37`, but **commit existence alone is not a substitute** for a retained test log or CI URL.

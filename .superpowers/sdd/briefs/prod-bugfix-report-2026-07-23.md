# Production bugfix report — 2026-07-23

Branch: `feature/bar-c`

Local verification limit: `cargo check --workspace` reaches dependency compilation but
Windows Application Control blocks the generated `serde` build script with OS error
4551. `anchor` is not installed. Rustfmt, metadata, diff, and source-policy checks pass;
Linux CI now owns the full Cargo/Anchor gate.

## Critical

| ID | Status | Fix evidence |
|---|---|---|
| C1 | FIXED | Signed V0 RPC `simulateTransaction`, real blockhash, live balance/fee checks, explicit send gate: `b38a94a`, `a27dd1d`, `8e7fd10`; `crates/supersonic-tx-cli/src/main.rs`, `crates/supersonic-tx-sdk/src/sign.rs` |
| C2 | FIXED | Default builder is router-free; `--via-router` verifies executable loader ownership: `8e7fd10`; `builder.rs`, `sign.rs`, `README.md` |
| C3 | FIXED | Exactly one CPI is required, missing/non-executable targets fail, success event follows invoke; runtime tests added: `b13cacb`, `c4ac642`, `2e5997a`; `programs/supersonic-tx/src/lib.rs`, `tests/router_tests.rs` |
| C4 | FIXED | ALT account is fetched, owner/contents decoded, and failures use non-ALT fallback: `8e7fd10`, `b38a94a`; `sdk/src/alt.rs`, `cli/src/main.rs` |

## Important

| ID | Status | Fix evidence |
|---|---|---|
| I1 | FIXED | Complete signing plus simulate-first `--send`; default signatures rejected: `8e7fd10`, `b38a94a` |
| I2 | FIXED | Keygen/I/O/fund/drain/underfund checks and CLI handoff consumption: `69d1e91`, `0c36e31`, `1f1eb31`, `b38a94a` |
| I3 | FIXED | Typed campaign planner, default intent isolation, best-effort decoys, fatal real intent: `8e7fd10`, `b38a94a` |
| I4 | FIXED | Builder requires validated sinks or explicit no-transfer profile; CLI reports disabled transfer noise: `8e7fd10` |
| I5 | FIXED | Every atomic sink/tip requires RPC proof of system ownership and non-executability; allowlist/cooker provenance alone is rejected: `8e7fd10`, `681be5c`; `noise.rs`, `builder.rs` |
| I6 | FIXED | `BuiltBundle` returns the exact final manifest/message/size after shrink: `8e7fd10`; `builder.rs` |
| I7 | FIXED | Shrink removes CU price before preserving the CU limit; exhaustion test added: `8e7fd10`; `builder.rs` |
| I8 | FIXED | Public placeholder-signature transaction builder removed; defaults remain size-estimation-only: `8e7fd10` |
| I9 | FIXED | Router count must equal the one executed CPI; event reports `routed_instruction_count`: `b13cacb`, `2e5997a` |
| I10 | FIXED | Cluster, timestamp, pubkeys, uniqueness, fee-payer count, secrets, and balances validated: `7781d77`; `account-cooker/src/types.rs` |
| I11 | FIXED | README, architecture, deploy guide, and CLI info now state implemented behavior and limitations: `46d1a17`, `b38a94a` |
| I12 | PARTIAL | Program-runtime tests and Linux Cargo/Anchor CI added (`b13cacb`, `46d1a17`), but this host cannot execute them due WDAC/missing Anchor and no remote CI result exists yet. |
| I13 | FIXED | Safe builder no longer exposes arbitrary custom generators; `DecoyKind::Custom` removed: `8e7fd10` |

## Minor disposition

- M1, M3, M4, M5 fixed opportunistically by typed Clap levels, saturating CU math,
  deterministic program ID parsing, and drive-relative/ADS path rejection.
- M2 deferred: the public Benford sampler still has a rejection loop. Built-in bounds
  are valid and bounded in product use; changing its public return type is outside the
  Critical/Important scope.

## Required release gate

Do not claim bar-C complete or use real SOL until the new Linux workflow passes
`cargo test --workspace --locked` and `anchor build`, and any router use has a recorded
deployment in `docs/deploy.md`.

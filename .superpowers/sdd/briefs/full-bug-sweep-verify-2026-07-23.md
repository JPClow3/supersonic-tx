# Full bug sweep verification — 2026-07-23

Branch: `feature/bar-c`  
HEAD: `bd64492` (includes fix commits `8c8bbe3`, `9e4099a`, `b3fe184`, `0ac4902`, `7cfb2b1`, `bd64492`)  
Method: read-only source review of `programs/`, `crates/`, tracked docs vs sweep brief and fix report.

## Summary

| Severity | Verified FIXED | Still open (source) | Deferred (external) |
| --- | --- | --- | --- |
| Critical | 1 | 0 | 0 |
| Important | 13 | 0 | 1 (I12) |

No Critical or Important source-level regressions found. I12 remains an external release gate (no remote CI, Anchor, deployment, or smoke evidence).

## Finding table

| ID | Status | Evidence |
| --- | --- | --- |
| **C1** | **FIXED** | `write_keypair_file_new` uses `create_new(true)` (`cooker.rs:393-408`); existing paths return `KeyFileExists` before write (`cooker.rs:118-120`); rollback on partial failure (`cooker.rs:125-133`); regression test `second_write_refuses_to_change_existing_key_bytes` (`cooker.rs:482-504`). |
| **I1** | **FIXED** | Campaign prebuilds all txs with live fee/spend (`main.rs:591-611`); `real_reserve` from real-intent spend+fee (`main.rs:613-617`); decoys skipped when `decoy_preserves_reserve` fails (`main.rs:361-362`, `626-632`); unit test `campaign_skips_decoy_at_real_intent_reserve_boundary` (`main.rs:750-752`). |
| **I2** | **FIXED** | Campaign compiles via `FuzzyBundleBuilder::build_manifest_bundle` (`main.rs:594-599`, `642-647`); shared shrink loop in `builder.rs:201-228`. |
| **I3** | **FIXED** | `resolve_keypairs` returns `(usize, Keypair)`, skips pathless `DrainTarget` (`cooker.rs:165-185`); CLI loads by account index (`main.rs:180-192`); test `pathless_drain_target_is_skipped_before_keypair_resolution` (`cooker.rs:543+`). |
| **I4** | **FIXED** | `--via-router` wraps target in `routed_instruction` CPI (`main.rs:281-284`, `320-337`); on-chain CPI test `execute_fuzzy_bundle_system_transfer_cpi` (`router_tests.rs:175+`); commit `bd64492`. |
| **I5** | **FIXED** | Parameter renamed to `routed_instruction_count`; rejects `!= 1` (`lib.rs:32-42`, `77-81`); docs/logs describe one routed CPI (`lib.rs:28-31`, `44-48`). |
| **I6** | **FIXED** | `verify_rpc_cluster` compares RPC genesis to declared cluster (`main.rs:222-246`); invoked on cook (`main.rs:485`) and handoff load (`main.rs:177`). |
| **I7** | **FIXED** | `fund_accounts` rejects sponsor/cooker/handoff mismatch before RPC (`cooker.rs:195-201`); test asserts `SponsorMismatch` (`cooker.rs:576`). |
| **I8** | **FIXED** | `write_keypair_dir` validates pair/account metadata (`cooker.rs:98-107`); clones source accounts preserving `funded_lamports` / `min_required_lamports` (`cooker.rs:136-144`); test `write_keypair_dir_handles_sink_first_ordering` (`cooker.rs:444-478`). |
| **I9** | **FIXED** | `--drain-to` requires `--send` and `--handoff` (`main.rs:573-574`, `665-684`); drain errors reported separately from campaign success (`main.rs:679-683`). |
| **I10** | **FIXED** | Keyless `assemble` with optional payer/target (`main.rs:74-84`, `375-408`); `simulate` remains signed RPC path (`main.rs:85-100`, `526-540`); clap tests document split (`main.rs:756-771`). |
| **I11** | **FIXED** | Windows private-dir + `icacls` procedure and custody warnings in `README.md:50-59`. Runtime ACL preflight before funding not implemented (docs-only scope per fix report). |
| **I12** | **DEFERRED** | `.github/workflows/ci.yml` present; no remote/upstream, green Linux CI, Anchor build, deployment, or smoke signature in repo. WDAC still blocks local `cargo check`. |
| **I13** | **FIXED** | ALT lifecycle checks in `alt.rs:47-56`; cast ALT fallback (`main.rs:440-457`); campaign ALT fallback (`main.rs:640-651`). |
| **I14** | **FIXED** | Tracked: `docs/superpowers/specs/2026-07-23-supersonic-tx-design.md`, `docs/superpowers/plans/2026-07-23-supersonic-tx.md`, sweep briefs (git ls-files). |

## Must-fix (before real SOL / Bar C release)

1. **I12 (external):** Push branch, obtain green Linux `cargo test --workspace --locked` + `anchor build`, deploy router, record smoke signature.
2. **Optional hardening (not sweep blockers):** runtime Windows ACL preflight (I11 docs-only gap); local WDAC prevents compile-time proof on this host.

## Verification constraints

- No runtime test execution (WDAC error 4551 on build scripts).
- Anchor not installed locally.
- Source-level claims verified by file:line inspection only.

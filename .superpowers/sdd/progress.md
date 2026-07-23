# SDD Progress Ledger

Branch: feature/bar-c
Started: 2026-07-23
Plan: docs/superpowers/plans/2026-07-23-supersonic-tx.md
HEAD: 59abc37

Task 1: complete (commits 0cc2355, review clean)
Task 2: complete (commits 0cc2355..6b7e922, review clean); minor/important for final: untracked workspace members at Task 2 HEAD; WDAC blocks full cargo check
Task 3: complete (commits 6b7e922..67fed6c, review clean); program id GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9
Task 4: complete (commits 67fed6c..608c459, review clean after CLI fix); minors: CLI diag mismatch, no runtime --send refusal test
Task 5: complete (commits 3bb36a2, 3e003ec; source re-review clean); sink validation and known-program deny-list present; no unsafe public `StatisticalTransferNoise::new(Vec<Pubkey>)` or `from_sinks(Vec<Pubkey>)`; tests not runtime-proven because WDAC blocks Cargo
Task 6: complete (commit a11151e); shrink priority test added for statistical â†’ memo ordering with CU retention; focused Cargo test could not run because cargo is unavailable locally

Final review minors:
- Task 4: CLI diagnostic mismatch and no runtime `--send` refusal test.
- Task 5: cooked provenance is documented rather than type-enforced; static deny-list cannot replace RPC executable-account validation; invalid sinks are silently discarded by `with_sinks`.
- Current CLI still fabricates in-memory ALT contents; unfinished Tasks 13/15.

Verification constraints:
- WDAC blocks Rust build-script execution on this host.
- Linux Rust gates documented green in commit `59abc37` report (no `.log` in repo); OOM re-run without Docker volumes failed on this host.
- Anchor build, remote CI, deploy, and smoke: no evidence in repo.
Task 6: complete (commits 3e003ec..a11151e, review clean); minors: FromStr import scope, weaker identity asserts, only one shrink tier tested
Task 7: complete (commits a11151e..794a09d incl. validation fixes; HEAD later 28d4d9b may include Task 5 seal); review clean; minor: cluster string unrestricted (devnet/mainnet-beta/localnet not enum-enforced)
Task 5: complete (through 28d4d9b seal; review clean) â€” tip/cooker provenance only

## Prod bug hunt 2026-07-23

Source: `.superpowers/sdd/briefs/prod-scenario-bug-hunt-2026-07-23.md` (branch `feature/bar-c`, snapshot `f0dcad8`)

**Finding counts:** 4 Critical Â· 13 Important Â· 5 Minor (22 total) Â· **Overall risk: CRITICAL â€” do not use with real SOL**

### Top 5 blockers

1. Implement real signer, RPC blockhash/balance checks, simulation, gated send; stop calling offline estimation â€œsimulationâ€
2. Remove synthetic ALT; fetch/verify on-chain lookup tables
3. Deploy/verify router program or disable router noops by default
4. Fix `execute_fuzzy_bundle` silent CPI skip and dishonest success events/counts
5. Finish account-cooker + campaign isolation + green Linux CI before real funds

### Critical â†’ Task mapping (plan tasks 8â€“20)

| ID | Summary | Task |
| --- | --- | --- |
| C1 | Fake simulate/dry-run cast (no RPC, fake blockhash, false PASSED) | **15** (sign/simulate via **14**) |
| C2 | Every default level depends on unproven router deployment | **15** (`--via-router` default off; deploy smoke **12**) |
| C3 | `execute_fuzzy_bundle` reports success without executing CPI | **11** |
| C4 | `--alt` fabricates lookup-table contents | **13** + **15** |

### UNCOVERED

- **C3 (partial):** Task 11 covers CPI program tests but does not explicitly require rejecting empty/non-executable `remaining_accounts` before emitting success â€” add lib.rs guard when implementing Task 11.

### Important (summary)

- I1 cast `--send` refused / feature missing Â· I2 cooker schema-only Â· I3 no campaign Â· I4 silent missing transfer noise Â· I5 sink provenance weak Â· I6 manifest/compile mismatch Â· I7 CU-limit shrink gap Â· I8 unsigned tx export Â· I9 dishonest `decoy_count` Â· I10 weak handoff validation Â· I11 docs overclaim Â· I12 no runtime test/CI Â· I13 custom decoy bypass

Companion mapping: `.superpowers/sdd/briefs/bug-hunt-task-mapping.md`

## Production bugfix pass

Critical: 4 FIXED, 0 remaining. Important: 12 FIXED, 1 PARTIAL (I12: Linux Rust subset in `59abc37` report; full gate incomplete). Added signed RPC simulation/send, real ALT resolution, router-free
defaults with executable deployment checks, honest CPI execution/events, strict cooker
handoffs, campaign isolation, runtime test source, Linux CI, and honest product docs.

Commits: `8e7fd10`, `a27dd1d`, `b13cacb`, `2e5997a`, `681be5c`,
`7781d77`, `d05f5e9`, `46d1a17`
plus cooker/CLI commits `0c36e31`, `1f1eb31`, `b38a94a`.

Local blocker: WDAC error 4551 blocks generated Rust build scripts; Anchor is absent.
Full status: `.superpowers/sdd/briefs/prod-bugfix-report-2026-07-23.md`.
Task 8: complete (commits 28d4d9b..050c203, review clean after panic/path fix)
Task 9: complete (commits 050c203..1f1eb31, review clean); minors: unused _handoff_dir, /tmp in duplicate-pubkey test
Task 10: complete (commits 1f1eb31..7e41014, review clean); minors: cook clap coverage, dry-run advisory only
Task 11: complete (commits 7e41014..7845e55, review clean); minor: brittle BanksClientError string match; C3 CPI honesty addressed

## Prod bugfix reconciliation

Evidence basis: `prod-bugfix-report-2026-07-23.md`, the committed source tree, and
recent git history. Statuses below follow the task numbering in the checked plan.

| Task | Status | Evidence / qualification |
| --- | --- | --- |
| 12 Deploy path documentation | DONE | `docs/deploy.md`, README link, commit `19497ef` (follow-up to `46d1a17`) |
| 13 AltResolver | DONE | Real RPC account fetch, ALT owner/state decode, CLI non-ALT fallback; `alt.rs`, `8e7fd10` |
| 14 Signing + real simulate/send | DONE | `sign.rs` signs and rejects default signatures; RPC simulation and gated send; `8e7fd10`, `a27dd1d` |
| 15 CLI cast/simulate honesty | DONE | Live blockhash/fee/balance checks, handoff/keypair loading, real ALT fallback, `--send`; `b38a94a`, `a27dd1d`, `8e7fd10` |
| 16 Campaign planner | DONE | `campaign.rs`, isolated real intent and validated decoy planning; `8e7fd10` |
| 17 CLI campaign | DONE | Campaign command, default isolation, best-effort decoys/ fatal real intent; `8e7fd10`, `b38a94a` |
| 18 CI + lockfile | DONE (qualified) | Docker Linux formatting and locked workspace tests passed; native program fallback build passed; Anchor/SBF build not reproduced |
| 19 Honest docs | DONE | README/ARCHITECTURE/info limitations and deployment link; `46d1a17`, `19497ef` |
| 20 Bar C verification | PARTIAL / BLOCKED | Docker Linux native suite is green (56 tests); Anchor/SBF build and deployment smoke remain unverified |

Remaining TODO for the SDD controller:
1. Obtain and record a real `anchor build`/SBF artifact (locked Linux Cargo tests are green).
2. Deploy the router on the intended cluster and record the program ID plus smoke result.
3. Re-run Task 20 deployment checks after those gates; do not claim bar-C/real-SOL readiness before then.

Note: the user callout labels docs as Task 18 and CI as Task 19, but the checked
plan assigns CI to Task 18 and docs to Task 19.
Task 12: complete (commit 19497ef, review clean/Approved); notes: local anchor build skipped; deploy.md omitted keypair-never-commit warning (restore in final docs polish if needed)

## Full bug sweep fix â€” 2026-07-23

Critical C1 and 13 source-level Important findings fixed in `8c8bbe3`,
`9e4099a`, `b3fe184`, and `0ac4902`. Fixes cover non-destructive cooker keys,
metadata/sponsor/cluster/key-resolution invariants, campaign reserve/MTU/drain,
real router CPI semantics, ALT lifecycle fallback, keyless assembly, provenance,
Windows custody guidance, and tracked acceptance documents.

Important I12 remains **PARTIAL**: committed Docker Linux test narrative (`59abc37`); no test log, no green Actions artifact, Anchor/deploy/smoke absent; WDAC still blocks Windows Cargo. Do not claim release readiness until
those gates pass. Full mapping:
`.superpowers/sdd/briefs/full-bug-sweep-fix-report-2026-07-23.md`.

## Full bug sweep verify â€” 2026-07-23

Source read-only verification at `bd64492`: **C1 FIXED** (create-new key writes +
byte-preservation test). **Important 13/14 FIXED in source** with file:line evidence;
**I12 PARTIAL** (Linux Rust subset in `59abc37`; CI/Anchor/deploy/smoke still open). No Critical or Important source
regressions. Must-fix before release: I12 only. Report:
`.superpowers/sdd/briefs/full-bug-sweep-verify-2026-07-23.md`.

## Tasks 18/20 / I12 reconciliation — 2026-07-23

Reconciled: commits `229f93f`, `59abc37`; report restored from git after disk overwrite. **Task 18 DONE (qualified)**; **Task 20 PARTIAL/BLOCKED**; **I12 PARTIAL** (not fully BLOCKED). No saved test log; later Docker OOM without volume setup failed.
Reports: `.superpowers/sdd/briefs/task-18-20-verify-reconciled.md`, `.superpowers/sdd/briefs/task-18-20-verify-report.md`.

## Task 20 Anchor/SBF attempt — 2026-07-23 (evening)

Docker: AVM installed Anchor **0.30.1** + Solana **1.18.26**; also tried `backpackapp/build:v0.30.1`. Added workspace `[profile.release] overflow-checks = true` (required by Anchor). **`anchor build` still fails**: SBF Cargo 1.75–1.79 vs `Cargo.lock` v4 and crates.io **edition2024** manifests. No `supersonic_tx.so`; README deployments still empty. Task 20 / I12 remain **PARTIAL/BLOCKED**. Details: `.superpowers/sdd/briefs/task-18-20-verify-report.md`, status: `.superpowers/sdd/briefs/bar-c-status-2026-07-23.md`.

## Bar C finish pass — 2026-07-23

- Restored full root workspace members (program, core, sdk, cli, account-cooker); router tests in excluded `programs/supersonic-tx-tests/`.
- Docker Linux re-run: **56** tests green (50 workspace + 6 router); log `.superpowers/sdd/briefs/bar-c-cargo-test-2026-07-23.log`.
- `target/deploy/supersonic_tx.so` retained (196640 bytes). Full `anchor build` still fails IDL/metadata tail (edition2024); log `.superpowers/sdd/briefs/bar-c-anchor-build-2026-07-23.log`.
- **Task 18:** DONE (qualified) — fmt + locked tests reproduced with log.
- **Task 20 / I12:** PARTIAL — Rust green; Anchor IDL gate + deploy/smoke blocked (no RPC keys).
- Report: `.superpowers/sdd/briefs/bar-c-finish-report-2026-07-23.md`.

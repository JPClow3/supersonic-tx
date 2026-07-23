# SDD Progress Ledger

Branch: feature/bar-c
Started: 2026-07-23
Plan: docs/superpowers/plans/2026-07-23-supersonic-tx.md
HEAD: a11151e

Task 1: complete (commits 0cc2355, review clean)
Task 2: complete (commits 0cc2355..6b7e922, review clean); minor/important for final: untracked workspace members at Task 2 HEAD; WDAC blocks full cargo check
Task 3: complete (commits 6b7e922..67fed6c, review clean); program id GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9
Task 4: complete (commits 67fed6c..608c459, review clean after CLI fix); minors: CLI diag mismatch, no runtime --send refusal test
Task 5: complete (commits 3bb36a2, 3e003ec; source re-review clean); sink validation and known-program deny-list present; no unsafe public `StatisticalTransferNoise::new(Vec<Pubkey>)` or `from_sinks(Vec<Pubkey>)`; tests not runtime-proven because WDAC blocks Cargo
Task 6: complete (commit a11151e); shrink priority test added for statistical → memo ordering with CU retention; focused Cargo test could not run because cargo is unavailable locally

Final review minors:
- Task 4: CLI diagnostic mismatch and no runtime `--send` refusal test.
- Task 5: cooked provenance is documented rather than type-enforced; static deny-list cannot replace RPC executable-account validation; invalid sinks are silently discarded by `with_sinks`.
- Current CLI still fabricates in-memory ALT contents; unfinished Tasks 13/15.

Verification constraints:
- WDAC blocks Rust build-script execution on this host.
- Cargo/Anchor runtime verification cannot be completed locally; no green `cargo test --workspace` or `anchor build` claim.
Task 6: complete (commits 3e003ec..a11151e, review clean); minors: FromStr import scope, weaker identity asserts, only one shrink tier tested
Task 7: complete (commits a11151e..794a09d incl. validation fixes; HEAD later 28d4d9b may include Task 5 seal); review clean; minor: cluster string unrestricted (devnet/mainnet-beta/localnet not enum-enforced)
Task 5: complete (through 28d4d9b seal; review clean) — tip/cooker provenance only

## Prod bug hunt 2026-07-23

Source: `.superpowers/sdd/briefs/prod-scenario-bug-hunt-2026-07-23.md` (branch `feature/bar-c`, snapshot `f0dcad8`)

**Finding counts:** 4 Critical · 13 Important · 5 Minor (22 total) · **Overall risk: CRITICAL — do not use with real SOL**

### Top 5 blockers

1. Implement real signer, RPC blockhash/balance checks, simulation, gated send; stop calling offline estimation “simulation”
2. Remove synthetic ALT; fetch/verify on-chain lookup tables
3. Deploy/verify router program or disable router noops by default
4. Fix `execute_fuzzy_bundle` silent CPI skip and dishonest success events/counts
5. Finish account-cooker + campaign isolation + green Linux CI before real funds

### Critical → Task mapping (plan tasks 8–20)

| ID | Summary | Task |
| --- | --- | --- |
| C1 | Fake simulate/dry-run cast (no RPC, fake blockhash, false PASSED) | **15** (sign/simulate via **14**) |
| C2 | Every default level depends on unproven router deployment | **15** (`--via-router` default off; deploy smoke **12**) |
| C3 | `execute_fuzzy_bundle` reports success without executing CPI | **11** |
| C4 | `--alt` fabricates lookup-table contents | **13** + **15** |

### UNCOVERED

- **C3 (partial):** Task 11 covers CPI program tests but does not explicitly require rejecting empty/non-executable `remaining_accounts` before emitting success — add lib.rs guard when implementing Task 11.

### Important (summary)

- I1 cast `--send` refused / feature missing · I2 cooker schema-only · I3 no campaign · I4 silent missing transfer noise · I5 sink provenance weak · I6 manifest/compile mismatch · I7 CU-limit shrink gap · I8 unsigned tx export · I9 dishonest `decoy_count` · I10 weak handoff validation · I11 docs overclaim · I12 no runtime test/CI · I13 custom decoy bypass

Companion mapping: `.superpowers/sdd/briefs/bug-hunt-task-mapping.md`

## Production bugfix pass

Critical: 4 FIXED, 0 remaining. Important: 12 FIXED, 1 PARTIAL (I12 runtime
verification only). Added signed RPC simulation/send, real ALT resolution, router-free
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

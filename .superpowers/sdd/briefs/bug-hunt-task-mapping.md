# Bug hunt → plan task mapping (2026-07-23)

Branch: `feature/bar-c` · Snapshot: `f0dcad8` · Plan: `docs/superpowers/plans/2026-07-23-supersonic-tx.md`

**Counts:** 4 Critical · 13 Important · 5 Minor (22 total)

## Critical → Task (8–20)

| ID | Finding | Task | Notes |
| --- | --- | --- | --- |
| C1 | Fake simulate/dry-run cast (no RPC, fake blockhash, false PASSED) | **15** | Task 14 provides `simulate_and_send`; Task 15 wires honest CLI |
| C2 | Default levels depend on unproven router deployment | **15** | `--via-router` opt-in default off; pair with Task 12 deploy smoke |
| C3 | `execute_fuzzy_bundle` success without CPI execution | **11** | **UNCOVERED (fix):** plan tests CPI happy path but does not explicitly require rejecting empty/non-executable `remaining_accounts` — add lib.rs guard when implementing Task 11 |
| C4 | `--alt` fabricates lookup-table contents | **13** + **15** | Task 13 `AltResolver` fetch; Task 15 removes synthetic CLI branch |

## Important (short)

- **I1** `--send` safely refused; cast feature missing → 14, 15
- **I2** account-cooker schema-only, no cook/fund/cast path → 7–10
- **I3** No campaign isolation → 16, 17
- **I4** Standard/Paranoid omit promised transfer noise → 5 (done), 15
- **I5** Cooked sinks don't prove non-executable wallet → 5 (done), 9, 15
- **I6** Displayed manifest ≠ compiled message → 4 (done), 15
- **I7** MTU shrink may drop CU limit, keep price → 4, 6 (done)
- **I8** Public unsigned `VersionedTransaction` API → 4 (done), 14
- **I9** `decoy_count` assertion-by-caller → 11
- **I10** Handoff accepts unusable/contradictory accounts → 7 (done), 9
- **I11** Docs advertise forbidden/nonexistent behavior → 19
- **I12** No runtime program test or CI gate → 11, 18
- **I13** Custom decoy hooks bypass fail-soft → 5 (done), 16

## Top 5 blockers

1. Real signer + RPC blockhash/balance + simulation + gated `--send` (14, 15)
2. Remove synthetic ALT; fetch/verify on-chain tables (13, 15)
3. Deploy/verify router or disable router noops by default (12, 15)
4. Fix silent CPI skip + honest events/counts (11 — add explicit lib.rs fix)
5. Finish account-cooker + campaign isolation + green Linux CI (8–10, 16–18)

Full report: `.superpowers/sdd/briefs/prod-scenario-bug-hunt-2026-07-23.md`

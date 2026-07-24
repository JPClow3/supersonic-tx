# Full bug hunt fix report — 2026-07-23 (PM)

Branch: `feature/bar-c`  
Hunt brief: `.superpowers/sdd/briefs/full-bug-hunt-2026-07-23-pm.md`  
Verification: Docker `rust:latest` → `cargo test --workspace --locked` — **52** tests passed (exit 0).

| ID | Status | Fix | Commit |
| --- | --- | --- | --- |
| C1 | FIXED | Broadcast uses `send_and_confirm_transaction`; campaign drain runs only after confirmed real intent | `fix: confirm sends…` |
| I1 | FIXED | Campaign recompiles/signs each tx with a fresh blockhash immediately before simulate/send | same |
| I2 | FIXED | Handoff load passes `--rpc-url`; `localnet` requires localhost/127.0.0.1 (empty URL rejected) | same |
| I3 | FIXED | `fund_accounts` fetches a new blockhash before each funding transfer | `fix: refresh funding blockhash…` |
| I4 | FIXED | `from_cooker_decoy_sink` requires `secret_key_path`; ARCHITECTURE/README aligned with confirm + localnet | `fix: require secret path…` (+ docs in first commit) |
| M1 | DEFERRED | Dry-run still advertises configured `funded_lamports`; live checks usually fail-close | — |
| M2 | DEFERRED | Campaign lacks `--via-router` | — |
| M3 | DEFERRED | Unused `PostNoise` variant | — |
| M4 | DEFERRED | Benford unbounded rejection loop outside product bounds | — |
| M5 | DEFERRED | Dual-lock / public-cluster deploy external gates (untouched) | — |

Totals: **Critical 1/1 FIXED · Important 4/4 FIXED · Minor 0/5 fixed (all DEFERRED).**

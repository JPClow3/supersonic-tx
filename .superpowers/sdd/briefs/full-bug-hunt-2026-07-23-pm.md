# Full bug hunt — `supersonic-tx` HEAD (PM)

Date: 2026-07-23 (PM)  
Branch: `feature/bar-c` at `a413c70`  
Scope: Fresh walk of cook → simulate/cast/--send → campaign (isolate/MTU/reserves) → ALT → TrustedSystemAccount sinks → program CPI → docs vs code.

Prior AM sweep fixes (key overwrite, via-router CPI wrap, routed_instruction_count, sponsor match, pathless DrainTarget, MTU shrink on campaign manifests, etc.) were re-checked and are **not** re-filed.

## Counts

**1 Critical · 4 Important · 5 Minor (10 total).**

## Critical

### C1. Campaign `--send` ignores in-flight spends; `--drain-to` can race the real intent

- **Severity:** Critical
- **File:line:** `crates/supersonic-tx-sdk/src/sign.rs:76-82`; `crates/supersonic-tx-cli/src/main.rs:619-684`
- **Scenario:** `campaign --send --txs 2` (optionally `--drain-to`) with a fee payer that can afford one decoy + real intent on confirmed balance, but not all planned spends concurrently.
- **Why it fails:** `simulate_and_send` broadcasts with `send_transaction` and does **not** confirm. Campaign re-reads `get_balance` between txs (confirmed commitment), so successful but unconfirmed decoy spends are invisible. Multiple decoys + real intent can all pass the reserve check against the same balance and overdraw when they land. With `--drain-to`, drain runs immediately after a non-confirmed real-intent accept, and can empty the fee payer before the intent finalizes.
- **Suggested fix:** Confirm broadcasts (`send_and_confirm_transaction`), refresh blockhash/re-sign between campaign txs, and only run post-campaign drain after the real intent has confirmed.

## Important

### I1. Campaign signs every tx with one shared blockhash

- **Severity:** Important
- **File:line:** `crates/supersonic-tx-cli/src/main.rs:590-612,619-651`
- **Scenario:** `campaign --send` with several decoys; prior sends consume wall-clock / slots.
- **Why it fails:** All planned messages are compiled and signed once with a single `get_latest_blockhash` before the loop. Later txs can expire (`BlockhashNotFound`) after earlier sends. ALT fallback refreshes hash only on the retry path.
- **Suggested fix:** Rebuild + re-sign each campaign transaction with a fresh blockhash immediately before simulate/send (after any prior confirmed send).

### I2. Handoff consume treats empty `rpc_url` as satisfying `localnet`

- **Severity:** Important
- **File:line:** `crates/supersonic-tx-cli/src/main.rs:177`; `222-246`
- **Scenario:** Handoff `cluster: "localnet"` used with `cast`/`campaign`/`simulate` against any RPC whose genesis is neither devnet nor mainnet (e.g. testnet), or a remote non-main/dev validator.
- **Why it fails:** `load_accounts` calls `verify_rpc_cluster(rpc, &handoff.cluster, "")`. For `localnet`, an empty URL short-circuits the localhost/127.0.0.1 requirement, so any non-devnet/non-mainnet genesis matches.
- **Suggested fix:** Pass the operator `--rpc-url` into verification; require localhost/127.0.0.1 for `localnet` (never treat empty URL as local).

### I3. `fund_accounts` reuses one blockhash for every funding transfer

- **Severity:** Important
- **File:line:** `crates/account-cooker/src/cooker.rs:202-229`
- **Scenario:** `cook` with a large `--sinks` count; funding takes longer than blockhash validity.
- **Why it fails:** A single `get_latest_blockhash` is taken before the loop; each account transfer is `send_and_confirm`’d against that same hash. Mid-batch confirms can push later transfers past expiry.
- **Suggested fix:** Fetch a fresh blockhash before each funding transfer (or after each confirm).

### I4. Docs claim cooker sink minting requires a secret path; code does not

- **Severity:** Important (docs vs code / provenance honesty)
- **File:line:** `ARCHITECTURE.md:41-44`; `crates/supersonic-tx-sdk/src/noise.rs:197-213`
- **Scenario:** Operator or auditor trusts ARCHITECTURE that `from_cooker_decoy_sink` demands a secret path under the cook dir.
- **Why it fails:** Minting only checks `DecoySink` role, pubkey parse, and deny-list. Secret path is enforced later by handoff schema validation for cooker-produced JSON, but the SDK mint path itself will accept a pathless `DecoySink` account struct. Documentation overstates the gate.
- **Suggested fix:** Require `secret_key_path.is_some()` in `from_cooker_decoy_sink` (aligned with handoff rules) and keep the doc accurate.

## Minor

### M1. Dry-run handoffs still advertise configured `funded_lamports`

- **File:line:** `crates/supersonic-tx-cli/src/main.rs:502-505`; `crates/account-cooker/src/cooker.rs:292-317`
- **Notes:** Warning is advisory; `assert_funded_for_cast` trusts JSON. Live fee-payer balance / sink `get_account` usually fail-close for truly unfunded keys. Residual honesty gap.

### M2. Campaign has no `--via-router` while cast/simulate do

- **File:line:** `crates/supersonic-tx-cli/src/main.rs:129-155` vs `107-127`
- **Notes:** Docs do not claim campaign router wrapping; feature asymmetry only.

### M3. `PlannedTxKind::PostNoise` is never produced

- **File:line:** `crates/supersonic-tx-sdk/src/campaign.rs:8-12`
- **Notes:** Dead variant; no operator impact.

### M4. Benford sampler can spin if no digit range intersects `[min,max]`

- **File:line:** `crates/supersonic-tx-sdk/src/noise.rs:280-307`
- **Notes:** Product bounds `[1000,50000]` are safe; public helper still has an unbounded rejection loop.

### M5. Dual-lock / public-cluster deploy remain external gates

- **File:line:** `README.md:138-146`; `ARCHITECTURE.md:90-101`
- **Notes:** Documented; not a new source defect. Do not break dual-lock or `.so` paths in this pass.

## Verification notes (hunt time)

- Source review only for this brief; fixes will prefer Docker `cargo test --workspace --locked`.
- No keypairs/secrets inspected or staged.

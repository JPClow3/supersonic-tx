# Full bug sweep — `supersonic-tx` Bar C

Date: 2026-07-23  
Branch: `feature/bar-c` at `19497ef`  
Scope: Tasks 1–19, current `programs/`, `crates/`, CLI, docs, CI, and the real cook → handoff → cast/campaign paths.

## 1. Executive summary

**Risk: CRITICAL — do not use with real SOL yet.**

Counts: **1 Critical · 14 Important · 7 Minor (22 total).**

The bugfix pass removed the earlier dummy-signature, synthetic-ALT, default-router, sink-provenance, and silent-CPI failures. It also introduced or left several operator-path hazards. Most seriously, a second `cook` into the same output directory silently truncates the previous key files, potentially making already-funded accounts permanently inaccessible. Campaign “isolation” can spend the balance needed by the real intent, bypasses the SDK MTU shrink loop, and has no drain path. The branch still has no remote CI result or recorded deployment.

Verification was necessarily mixed:

- `cargo fmt --all -- --check` passed using the explicit Cargo binary.
- `cargo test --workspace --locked` was attempted and failed before project compilation because WDAC blocked the generated `serde` build script with OS error 4551. Cargo is not on normal `PATH`.
- `anchor build` could not run because `anchor` is not installed.
- There is no Git remote/upstream, so the workflow has no observable run for this branch.
- No tracked secret was found by source/history scans. A local ignored deploy keypair exists; its contents were not inspected.

## 2. Critical findings

### C1. Re-running `cook` can destroy the only copies of funded account keys

- **Severity:** Critical
- **Task:** 8, 9, 10
- **File:line:** `crates/supersonic-tx-cli/src/main.rs:312-331`; `crates/account-cooker/src/cooker.rs:90-114`; dependency implementation `solana-sdk-1.18.26/src/signer/mod.rs:156-170`
- **Scenario:** Run `cook --out-dir cooked`, fund/use its handoff, then run `cook --out-dir cooked` again.
- **Why it fails:** Reuse detection compares newly generated pubkeys with old files, which will almost never match. The same deterministic paths (`keys/fee_payer.json`, `keys/sink_0.json`, …) are then opened with `truncate(true)`. Old handoffs still reference those paths but the secrets now belong to new pubkeys. Funds held by the old keys can become permanently inaccessible.
- **Suggested fix:** Refuse any existing key path by default using create-new semantics; support an explicit, loudly named destructive override only if necessary. Prefer a unique per-cook directory and atomically write keys plus handoff. Test that a second cook cannot alter existing key bytes.

## 3. Important findings

### I1. Campaign decoys can consume the balance reserved for the real intent

- **Severity:** Important
- **Task:** 16, 17
- **File:line:** `crates/supersonic-tx-sdk/src/campaign.rs:74-95`; `crates/supersonic-tx-cli/src/main.rs:397-425`
- **Scenario:** A payer can afford the target plus the fixed handoff minimum, but not the target plus all `--txs` decoys, their transfers, and priority/base fees.
- **Why it fails:** Decoy transactions are sent first without a campaign-wide fee/spend calculation or reservation. Successful decoys reduce the payer balance; the later real-intent simulation/send then fails. Transaction separation therefore does not provide economic isolation.
- **Suggested fix:** Prebuild every transaction, calculate worst-case transfer amounts and live fees, reserve target amount plus real-intent fee, and cap/skip decoys that would touch the reserve. Add a regression test where the payer is deliberately near the reserve boundary.

### I2. Campaign bypasses the MTU shrink implementation

- **Severity:** Important
- **Task:** 6, 13, 16, 17
- **File:line:** `crates/supersonic-tx-cli/src/main.rs:404-413`; `crates/supersonic-tx-sdk/src/builder.rs:187-216`; `README.md:14-15`; `ARCHITECTURE.md:25-33`
- **Scenario:** A campaign target or isolated real transaction fits only after dropping optional memo/price padding, especially after ALT fallback.
- **Why it fails:** Campaign directly calls `compile_v0_message` and returns `TransactionSizeExceeded`; it never calls `build_bundle` or `shrink_decoys`. A real intent that could fit after shrinking fails fatally. README/architecture describe ALT fallback plus MTU shrink without this exception.
- **Suggested fix:** Represent each `PlannedTx` as target/decoy metadata and compile it through the same `build_bundle` shrink path used by cast. Return the final manifest for diagnostics.

### I3. A valid pathless `DrainTarget` handoff cannot be consumed by cast/campaign

- **Severity:** Important
- **Task:** 7, 8, 15, 17
- **File:line:** `crates/account-cooker/src/types.rs:132-145`; `crates/account-cooker/src/cooker.rs:134-156`; `crates/supersonic-tx-cli/src/main.rs:156-173`
- **Scenario:** Load a schema-v1 handoff containing one valid `DrainTarget` with `secret_key_path: null`, which validation intentionally permits.
- **Why it fails:** `resolve_keypairs` requires a secret path for every account before the CLI can ignore `DrainTarget`. Thus a handoff accepted by deserialization and drain policy is rejected by cast/campaign.
- **Suggested fix:** Resolve role-aware keypairs, skipping pathless `DrainTarget`, or return keyed `(account_index, keypair)` entries rather than a positional vector.

### I4. `--via-router` does not route the target through the CPI router

- **Severity:** Important
- **Task:** 11, 15
- **File:line:** `crates/supersonic-tx-cli/src/main.rs:217-230`; `programs/supersonic-tx/src/lib.rs:33-85`; authoritative spec §6.2/§16
- **Scenario:** Operator requests `cast --via-router` expecting the documented opt-in CPI wrapper.
- **Why it fails:** The target remains a direct System Program transfer. The flag only appends one or two `noop_decoy` calls. The `execute_fuzzy_bundle` CPI path is never constructed by the CLI/SDK.
- **Suggested fix:** Rename the current flag to `--router-noop`, or implement actual target wrapping with explicit account metas and instruction data. Do not preserve a flag whose name implies different execution semantics.

### I5. `execute_fuzzy_bundle` still describes a decoy count it never executes

- **Severity:** Important
- **Task:** 11
- **File:line:** `programs/supersonic-tx/src/lib.rs:28-43,51-83`
- **Scenario:** A caller invokes the public router with `decoy_count = 1` and one target CPI.
- **Why it fails:** The program executes exactly one routed CPI and no decoy instruction, yet the API parameter and log call it `decoy_count` and the doc says it receives/executes decoys. The event was made honest, but the public instruction contract remains semantically false.
- **Suggested fix:** Rename the parameter to `routed_instruction_count` (or remove it and derive one), rewrite the instruction docs/logs, and separate any future decoy list into a concrete validated representation.

### I6. Handoff cluster provenance is ignored by both cook and consumption

- **Severity:** Important
- **Task:** 10, 15, 17
- **File:line:** `crates/supersonic-tx-cli/src/main.rs:56-59,302-327,146-184`
- **Scenario:** `cook --cluster mainnet-beta` with the default devnet RPC, or cast a devnet handoff against a mainnet RPC.
- **Why it fails:** `cluster` is only copied into JSON. It is never matched to `rpc_url`, genesis hash, or operator intent, and no required spec warning is emitted. The handoff can falsely label where funding occurred and the same keys can be used unexpectedly on another cluster.
- **Suggested fix:** Resolve cluster identity from genesis hash, reject cook mismatches, and reject or require explicit confirmation on consume mismatches.

### I7. `fund_accounts` does not enforce sponsor identity

- **Severity:** Important
- **Task:** 9
- **File:line:** `crates/account-cooker/src/cooker.rs:27-35,79-86,159-194`
- **Scenario:** SDK caller constructs `Cooker::new_offline(A)` and calls `fund_accounts` with keypair B.
- **Why it fails:** The handoff records A as `sponsor_pubkey`, but funds are sent by B. Neither `self.sponsor_pubkey`, the handoff field, nor `sponsor.pubkey()` are compared. The provenance field and threat-model edge become forgeable/incorrect.
- **Suggested fix:** Require all three sponsor pubkeys to match before the first RPC call and test mismatches.

### I8. `write_keypair_dir` returns account metadata that disables funding checks

- **Severity:** Important
- **Task:** 8, 9
- **File:line:** `crates/account-cooker/src/cooker.rs:90-114,364-388`
- **Scenario:** Follow the public/test-shaped flow: generate, call `write_keypair_dir`, assign its returned accounts into the handoff, then serialize.
- **Why it fails:** Every returned account has `funded_lamports = 0` and `min_required_lamports = 0`. Validation passes and `assert_funded_for_cast` has no minimum to enforce. The existing unit test demonstrates this replacement.
- **Suggested fix:** Make writing return only paths/status, mutate the existing accounts while preserving financial metadata, or accept the source accounts and return exact updated copies.

### I9. Completed campaign CLI has no drain option

- **Severity:** Important
- **Task:** 17
- **File:line:** `crates/supersonic-tx-cli/src/main.rs:114-137,371-426`; `crates/account-cooker/src/cooker.rs:198-256`
- **Scenario:** Operator completes a campaign and requests the plan/spec’s optional post-campaign drain.
- **Why it fails:** The library drain exists, but `campaign` exposes neither `--drain` nor a destination and never invokes it. Task 17 is marked done despite a defining operator-path feature being absent.
- **Suggested fix:** Add an explicit drain destination policy, invoke drain only after the intended campaign phase, and make partial-drain failures visible without misreporting campaign success.

### I10. `simulate` does not implement its normative assemble-without-keys contract

- **Severity:** Important
- **Task:** 15
- **File:line:** `crates/supersonic-tx-cli/src/main.rs:70-90,146-154,255-285`; spec §15
- **Scenario:** Operator wants an unsigned assembly report with no handoff/keypair, or no target.
- **Why it fails:** Clap requires a target and runtime requires exactly one signer source. The output reports only byte/decoy counts, not the promised decoy ratio, CU, MTU fill, or Benford status. This is a real signed RPC simulation, but it is not the full advertised `simulate` mode.
- **Suggested fix:** Split `assemble` and signed RPC simulation explicitly, or implement the documented optional-key/optional-target branch and complete diagnostics.

### I11. Windows key-file protection is unspecified and not enforced

- **Severity:** Important
- **Task:** 8, 10, 19
- **File:line:** `crates/account-cooker/src/cooker.rs:90-104`; `README.md:24-44`
- **Scenario:** Cook secrets into a Windows directory with inherited broad ACLs.
- **Why it fails:** Solana’s writer applies mode `0600` only on Unix; the non-Unix path uses ordinary inherited permissions. The spec requires Windows handling to be documented, but product docs contain no ACL warning or command.
- **Suggested fix:** Document a private-directory prerequisite and Windows ACL procedure; where feasible, verify/refuse broadly accessible output directories before funding.

### I12. Bar-C release gates and deployment remain unproven

- **Severity:** Important
- **Task:** 3, 11, 18
- **File:line:** `.github/workflows/ci.yml:1-29`; `README.md:21-22,53-65`; `docs/deploy.md:1-10`
- **Scenario:** A bounty judge checks reproducibility or an operator enables router noise.
- **Why it fails:** Local Cargo is WDAC-blocked, Anchor is absent, the repository has no remote/upstream, no workflow result exists, and README records no deployment or smoke signature. Source test presence is not runtime evidence.
- **Suggested fix:** Push to a remote, retain a green Linux `cargo test --workspace --locked` and `anchor build`, deploy, and record cluster/program/signature evidence before claiming Bar C.

### I13. ALT lifecycle state is discarded, so fallback is incomplete

- **Severity:** Important
- **Task:** 13, 15, 17
- **File:line:** `crates/supersonic-tx-sdk/src/alt.rs:23-37`; `crates/supersonic-tx-cli/src/main.rs:187-204`
- **Scenario:** RPC returns a correctly owned but deactivated/stale ALT whose data still deserializes.
- **Why it fails:** The resolver copies only addresses and does not inspect lookup-table metadata such as deactivation/extension state. Fetch therefore “succeeds”; compilation uses it; a later simulation failure aborts instead of taking the documented non-ALT fallback.
- **Suggested fix:** Validate usable lifecycle state against the current slot/slot hashes where the SDK permits, and retry compilation/simulation without ALT when failure is attributable to lookup resolution.

### I14. The authoritative spec and plan are not part of the branch

- **Severity:** Important
- **Task:** 18, 19
- **File:line:** `docs/superpowers/specs/2026-07-23-supersonic-tx-design.md`; `docs/superpowers/plans/2026-07-23-supersonic-tx.md`; git worktree
- **Scenario:** Judge or controller checks out `feature/bar-c` from a clean clone.
- **Why it fails:** Both authoritative documents are untracked, as are the midflight/remainder briefs. The checked-out branch cannot reproduce the stated acceptance contract or audit trail.
- **Suggested fix:** Commit the authoritative spec/plan (and intended final reports), or relocate the acceptance contract into tracked product documentation.

## 4. Minor findings

### M1. Public Benford sampler can loop forever

- **Severity:** Minor
- **Task:** 5
- **File:line:** `crates/supersonic-tx-sdk/src/noise.rs:258-281`
- **Scenario:** SDK caller supplies reversed or unreachable bounds.
- **Why it fails:** An unbounded rejection loop has no input validation or attempt limit.
- **Suggested fix:** Return `Result`, validate the range, and use a bounded/fallback sampling strategy.

### M2. Package metadata overclaims protection

- **Severity:** Minor
- **Task:** 19
- **File:line:** `Cargo.toml:18`
- **Scenario:** Crates/repository metadata is displayed independently of README.
- **Why it fails:** “defeat on-chain analytics, MEV front-runners” is materially stronger than the approved “partial behavioral obscurity” threat model.
- **Suggested fix:** Reuse the restrained README mission statement.

### M3. Program unit test contradicts the enforced count

- **Severity:** Minor
- **Task:** 11
- **File:line:** `programs/supersonic-tx/src/lib.rs:132-139`
- **Scenario:** Maintainer reads the inline test as instruction-contract evidence.
- **Why it fails:** It calls count `3` valid while production accepts only `1`. Runtime tests are stronger, but this stale test teaches the wrong invariant.
- **Suggested fix:** Delete the tautological test or assert the actual validation through one shared pure helper.

### M4. `PostNoise` is a public but unreachable campaign state

- **Severity:** Minor
- **Task:** 16
- **File:line:** `crates/supersonic-tx-sdk/src/campaign.rs:7-12,74-97`
- **Scenario:** SDK consumer expects planner output for every public enum variant.
- **Why it fails:** The planner never constructs `PostNoise`; execution semantics exist only as dead surface.
- **Suggested fix:** Implement post-noise explicitly or remove the variant until supported.

### M5. Public program-ID helper can panic

- **Severity:** Minor
- **Task:** 3
- **File:line:** `crates/supersonic-tx-core/src/lib.rs:8-13`
- **Scenario:** Future constant edit introduces an invalid base58 ID.
- **Why it fails:** A public helper panics instead of failing at compile time or returning a typed error.
- **Suggested fix:** Use a compile-time pubkey macro/static constant and retain the sync test.

### M6. Deploy guide omits key custody and complete cluster setup

- **Severity:** Minor
- **Task:** 12
- **File:line:** `docs/deploy.md:1-10`
- **Scenario:** Operator follows the document from a fresh machine.
- **Why it fails:** It does not explicitly say never to commit the deploy keypair, does not set/verify Solana CLI cluster/wallet, and asks for evidence without defining the fields to record.
- **Suggested fix:** Add custody warning, exact config/verification commands, loader-owner check, and a deployment evidence template.

### M7. Core public comments retain obsolete marketing language

- **Severity:** Minor
- **Task:** 5, 19
- **File:line:** `crates/supersonic-tx-core/src/types.rs:5-35`
- **Scenario:** SDK docs are generated from Rust comments.
- **Why it fails:** “Maximum obscurity,” “fool graph analytics,” “equalize TX profiles,” and “real or simulated” transfers do not match the honest current threat model and real-transfer-only implementation.
- **Suggested fix:** Rewrite API docs around density/configuration and explicit limitations.

## 5. Previously fixed items still holding

- **Unsigned/default broadcast:** `Signature::default()` remains only in `builder.rs:175-184` for serialized-size estimation. `sign.rs:13-35,60-82` requires all non-default signatures before simulation/send.
- **Synthetic ALT:** `alt.rs:12-37` fetches the on-chain account, verifies ALT-program ownership, and deserializes real addresses. No synthetic `AddressLookupTableAccount` construction exists in CLI.
- **Default router dependency:** `builder.rs:36-48` defaults to compute/memo only; CLI verifies an executable loader-owned program before adding router noops.
- **Sink provenance:** CLI resolves every sink through RPC; `noise.rs:138-163` rejects executable or non-System-owned accounts.
- **Manifest honesty and cast MTU:** `builder.rs:187-216` compiles and reports one final post-shrink manifest; targets are never removed.
- **CPI silent-success bug:** `programs/supersonic-tx/src/lib.rs:51-83` rejects missing/non-executable targets, invokes before emitting, and emits routed count `1`.
- **Drain rent preservation:** `account-cooker/src/cooker.rs:206-253,337-341` subtracts live fee and rent-exempt minimum.
- **Handoff structural validation:** schema, cluster, timestamps, pubkeys, uniqueness, fee payer, paths, and declared minimums are validated in `account-cooker/src/types.rs:85-147`.
- **Program ID sync:** `declare_id!`, core constant, and localnet/devnet `Anchor.toml` entries all use `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`.
- **Secret scan:** no tracked raw key or logging of key bytes was found; deploy keypair is ignored. This does not cure C1 or Windows ACL risk.

## 6. Recommended fix order for controller

1. Stop key overwrite immediately; add non-destructive/atomic cooker output tests.
2. Make campaign reserve target funds and use the shared MTU shrink path.
3. Repair handoff role-aware key resolution and sponsor/cluster invariants.
4. Decide router semantics: implement real `--via-router` CPI or rename it, then clean the on-chain API language.
5. Finish campaign drain and truthful simulate/operator diagnostics.
6. Harden/document secret-file permissions and ALT lifecycle fallback.
7. Commit authoritative docs, push the branch, obtain green Linux Cargo/Anchor gates, deploy, and record smoke evidence.

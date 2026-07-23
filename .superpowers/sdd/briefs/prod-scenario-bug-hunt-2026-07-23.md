# Production scenario bug hunt — `supersonic-tx` bar C

Date: 2026-07-23  
Branch inspected: `feature/bar-c`  
Committed snapshot: `f0dcad8` (`fix: validate handoff schema_version and relative secret_key_path`)  
Scope: current HEAD, observed worktree, recent commits, authoritative spec/plan, CLI → SDK → program paths.

## Verdict

**Overall risk: CRITICAL / do not use with real SOL.** The current CLI is an offline message estimator, not a signer, simulator, sender, cooker consumer, or campaign runner. `cast --send` is safely refused and no repository-owned RPC broadcast call exists, but the dry-run output materially overstates what was validated. The on-chain CPI wrapper also has a success-without-execution path that emits an apparently successful event.

Finding count: **4 Critical, 13 Important, 5 Minor (22 total).**

## Verification limits and positive checks

- `cargo check --workspace` was attempted through `C:\Users\lives\.cargo\bin\cargo.exe`. Windows Application Control/WDAC blocked generated Rust build scripts (`serde ... build-script-build`, OS error 4551), so no Rust target compiled.
- `anchor` is not on PATH, so `anchor build` and program-runtime tests could not run.
- No `.github/workflows/ci.yml`, deploy guide, recorded deployment, or runtime program test is tracked.
- The Solana 1.18 V0 API is currently correct: `v0::Message::try_compile` is used.
- Program IDs are textually synchronized: `declare_id!`, `PROGRAM_ID_STR`, and both `Anchor.toml` entries are `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`.
- No CLI/SDK call to `send_transaction`, `send_and_confirm_transaction`, or `simulate_transaction` exists. `cast --send` returns an error before broadcast.
- The handoff schema contains paths, not raw secret bytes, and no current product log prints secret material. Key generation, file permissions, loading, and funding are not implemented.

## Critical findings

### C1. “Simulate” and dry-run “cast” do not simulate anything

- **Severity:** Critical
- **Scenario:** 1, 3 — first devnet cast; missing/underfunded keypair; bad RPC.
- **File:line evidence:** `crates/supersonic-tx-cli/src/main.rs:102-127`, `127-156`, `158-217`
- **Why it fails in prod:** Missing key material creates an unfunded random payer, both commands use `Hash::new_unique()` instead of an RPC blockhash, and `simulate` has no RPC URL or RPC client. The supplied cast RPC URL is only printed. Bad URLs, zero balances, missing deployment, invalid blockhash, signature failure, and instruction failure all still produce “Bundle assembled successfully” or a “Simulation Report” with “PASSED”/“HIGH” claims.
- **Suggested fix:** Rename the current behavior to `assemble` until real signing exists. Implement blockhash fetch, complete signing, `simulateTransaction`, balance/fee checks, and truthful failure propagation before calling this simulation.

### C2. Every default level includes an unproven router deployment dependency

- **Severity:** Critical
- **Scenario:** 1, 7, 9 — Light/Standard/Paranoid cast and `noop_decoy`.
- **File:line evidence:** `crates/supersonic-tx-sdk/src/builder.rs:30-43`; `crates/supersonic-tx-sdk/src/noise.rs:319-368`; `programs/supersonic-tx/src/lib.rs:3-26`; `Anchor.toml:7-11`
- **Why it fails in prod:** The default builder always installs `AnchorRouterNoise`, and every level emits at least one router instruction. Textual ID sync does not prove that this generated local ID is deployed on devnet. If the account is absent or not executable, the atomic transaction fails and the real SOL transfer rolls back.
- **Suggested fix:** Make router noise conditional on an RPC-verified executable account owned by the loader, or allow a router-free fail-soft path. Publish and smoke-test an actual devnet deployment before enabling it by default.

### C3. `execute_fuzzy_bundle` can report success without executing the requested CPI

- **Severity:** Critical
- **Scenario:** 7 — on-chain CPI via `remaining_accounts`; event honesty.
- **File:line evidence:** `programs/supersonic-tx/src/lib.rs:50-78`, `80-88`
- **Why it fails in prod:** Empty `remaining_accounts`, or a non-executable first remaining account, silently skips the CPI. The handler then emits `BundleExecuted` and returns `Ok(())`, so an operator/indexer can observe a successful “bundle” even though the target intent never ran.
- **Suggested fix:** Reject missing/non-executable target programs whenever instruction data requests a CPI, and emit success only after an actual successful invocation. Use a separate explicit no-CPI mode/event if needed.

### C4. `--alt` fabricates lookup-table contents

- **Severity:** Critical
- **Scenario:** 4 — fake ALT flag.
- **File:line evidence:** `crates/supersonic-tx-cli/src/main.rs:117-125`
- **Why it fails in prod:** The CLI trusts the supplied ALT pubkey but invents its addresses as `[target, payer]`; it never fetches or deserializes the on-chain ALT. A compiled message can therefore reference indices whose real on-chain addresses differ, causing wrong-account resolution or rejection. Broadcast is blocked today, but this becomes immediately dangerous when signing is added.
- **Suggested fix:** Remove the synthetic ALT branch now. Fetch the ALT account through RPC, verify owner/state/address contents, and fall back to non-ALT compilation plus shrink on any mismatch.

## Important findings

### I1. `cast --send` is a safe refusal, but the advertised cast feature does not exist

- **Severity:** Important
- **Scenario:** 2 — cast with `--send`.
- **File:line evidence:** `crates/supersonic-tx-cli/src/main.rs:53-55`, `147-156`; `docs/superpowers/specs/2026-07-23-supersonic-tx-design.md:203-204`, `326-330`, `536`
- **Why it fails in prod:** `--send` always exits `Unsupported`; there is no signing helper or RPC sender. This prevents unsigned broadcast internally, but contradicts the normative CLI and SDK contract and means the core product cannot cast.
- **Suggested fix:** Keep the refusal until complete signer resolution and default-signature rejection land. Then simulate first and send only behind explicit `--send`.

### I2. account-cooker is a schema crate, not a production handoff

- **Severity:** Important
- **Scenario:** 3, 5 — load → fund → cast; underfunding; keypair paths.
- **File:line evidence:** `crates/account-cooker/src/lib.rs:1-2`; `crates/account-cooker/src/types.rs:5-184`; `crates/supersonic-tx-cli/src/main.rs:25-73`
- **Why it fails in prod:** There is no `Cooker`, key generation, key-file writer/loader, funding, drain, reuse detection, balance assertion, `cook` command, `--handoff` flag, or handoff-to-signer path. A valid schema-v1 file cannot be used to fund or cast anything.
- **Suggested fix:** Implement the cooker lifecycle and CLI wiring before documenting a handoff. Enforce actual RPC balances plus estimated target, decoy, priority-fee, and base-fee costs at cast time.

### I3. Campaign isolation and failure semantics are absent

- **Severity:** Important
- **Scenario:** 6 — multi-transaction campaign; decoy failure versus intent survival.
- **File:line evidence:** `crates/supersonic-tx-sdk/src/lib.rs:1-8`; `crates/supersonic-tx-cli/src/main.rs:25-73`
- **Why it fails in prod:** No campaign module, planner, CLI command, `isolate_intent`, or best-effort decoy loop exists. All currently assembled instructions share one atomic transaction, so any decoy failure kills the real intent.
- **Suggested fix:** Add typed `PlannedTxKind` separation and default intent isolation. Continue after decoy-only failure, but hard-fail and return nonzero for the real-intent transaction.

### I4. Standard/Paranoid builders silently omit their required transfer noise

- **Severity:** Important
- **Scenario:** 1, 10 — empty sinks; composable builder misuse.
- **File:line evidence:** `crates/supersonic-tx-sdk/src/builder.rs:30-43`, `46-57`, `85-116`; `crates/supersonic-tx-sdk/src/noise.rs:221-250`; spec `docs/superpowers/specs/2026-07-23-supersonic-tx-design.md:309-317`
- **Why it fails in prod:** The default builder does not install `StatisticalTransferNoise`, so Standard and Paranoid succeed with zero transfers despite the spec promising three and five. Empty sinks error only if callers explicitly invoke `with_sinks`; simply forgetting that call looks successful.
- **Suggested fix:** Encode level requirements in the builder and return a clear missing-sinks error for levels/configurations that promise transfer decoys. Alternatively expose an explicit `without_transfer_noise` profile and report it honestly.

### I5. “Cooked” sink provenance still does not prove a non-executable system wallet

- **Severity:** Important
- **Scenario:** 1, 3, 8, 10 — cooked/tip sinks; fail-soft transfer to program IDs.
- **File:line evidence:** committed `crates/supersonic-tx-sdk/src/noise.rs:88-128`; observed worktree `crates/supersonic-tx-sdk/src/noise.rs:104-165`; `crates/supersonic-tx-sdk/src/builder.rs:60-64`, `86-99`
- **Why it fails in prod:** HEAD exposes `assume_system_wallet` for any pubkey outside a small static deny-list. The observed follow-up narrows minting to cooker roles/allowlists, but still checks only role, base58, and the same deny-list; arbitrary executable accounts remain undetectable, and `RequireOnChainNonExecutable` is an always-error stub.
- **Suggested fix:** Resolve every sink through RPC and require `executable == false` plus the expected system ownership/state. Preserve provenance as a validated type carrying cluster/account evidence, not just a role label.

### I6. The displayed manifest is not the message that gets compiled

- **Severity:** Important
- **Scenario:** 1, 8 — dry-run diagnostics and MTU shrink.
- **File:line evidence:** `crates/supersonic-tx-cli/src/main.rs:111-128`, `136-145`, `168-181`; `crates/supersonic-tx-sdk/src/builder.rs:85-116`, `166-190`
- **Why it fails in prod:** The CLI calls `build_manifest()`, then `build_versioned_message()` calls it again. Random memo, CU price/limit, seeds, transfer amounts, and order are regenerated; shrink applies only to the second manifest. Reported counts, CU, entropy, and decoy composition can therefore describe a different bundle than the serialized message.
- **Suggested fix:** Build one manifest once, shrink that same manifest, compile it, and return both final manifest and message in one result object.

### I7. MTU shrink can drop the CU limit and retain only the priority price

- **Severity:** Important
- **Scenario:** 8 — 1232-byte shrink and CU retention.
- **File:line evidence:** `crates/supersonic-tx-sdk/src/noise.rs:241-273`; `crates/supersonic-tx-sdk/src/builder.rs:214-269`, especially `247-258`; test `builder.rs:332-358`
- **Why it fails in prod:** Compute noise emits both limit and price. The fallback protects only the last compute-budget instruction; after prior priorities are exhausted it can remove `SetComputeUnitLimit` while preserving `SetComputeUnitPrice`, violating the stated “keep at least one CU ix” intent if that means limit retention. The test verifies transfer removal only, not CU-limit survival.
- **Suggested fix:** Classify compute-budget opcodes and explicitly retain one `SetComputeUnitLimit`; treat price as lower priority. Add shrink-to-exhaustion tests covering both original instruction orders.

### I8. A production-shaped unsigned transaction API remains exported

- **Severity:** Important
- **Scenario:** 2, 10 — unsigned/default signatures; deprecated API use.
- **File:line evidence:** `crates/supersonic-tx-sdk/src/builder.rs:193-212`; `crates/supersonic-tx-sdk/src/lib.rs:4`
- **Why it fails in prod:** `build_versioned_transaction` returns a `VersionedTransaction` filled with `Signature::default()`. The repository does not send it, but downstream SDK users can pass the value to an RPC client because its type looks submit-ready; deprecation is only a warning.
- **Suggested fix:** Remove or make the wrapper non-public before release. Size estimation should return a byte count/message-only type that cannot be confused with a signed transaction.

### I9. `decoy_count` is assertion-by-caller, not validation

- **Severity:** Important
- **Scenario:** 7 — decoy-count validation and event honesty.
- **File:line evidence:** `programs/supersonic-tx/src/lib.rs:33-48`, `50-86`, `113-119`
- **Why it fails in prod:** The program only rejects zero. It neither receives nor executes `decoy_count` decoys, and accepts any nonzero value for the event, so `BundleExecuted.decoy_count` can claim 255 decoys while none ran.
- **Suggested fix:** Derive the count from a concrete validated decoy representation or remove the field. At minimum enforce a bounded count and prove correspondence before emitting it.

### I10. Handoff validation accepts unusable or contradictory accounts

- **Severity:** Important
- **Scenario:** 5 — schema v1, missing `secret_key_path`, cluster/role/balance validity.
- **File:line evidence:** `crates/account-cooker/src/types.rs:12-29`, `42-67` at HEAD (observed worktree extends construction validation but retains the same field checks)
- **Why it fails in prod:** `secret_key_path: None` is valid for every role, including `FeePayer`; pubkeys and sponsor are not parsed; empty accounts, duplicate fee payers/pubkeys, unsupported clusters, impossible timestamps, and `funded_lamports < min_required_lamports` are accepted. The schema can deserialize successfully but cannot support signing/funding.
- **Suggested fix:** Validate role-specific required paths, base58 pubkeys, one fee payer, uniqueness, cluster allowlist, and balance invariants. Keep `None` only for roles explicitly not expected to sign.

### I11. Product docs advertise forbidden and nonexistent behavior

- **Severity:** Important
- **Scenario:** 1, 2, 4, 5, 6, 10 — docs versus code.
- **File:line evidence:** `README.md:55`, `64-71`, `116-173`; `ARCHITECTURE.md:42-64`, `68-80`; `crates/supersonic-tx-cli/src/main.rs:211-237`
- **Why it fails in prod:** Docs claim RPC/Jito broadcast, ALT readiness, account-cooker composability, high-volume Jupiter/Raydium/Orca transfer sinks, CU normalization, and front-running mitigation. CLI `info` says “ACTIVE.” The code has no sender/cooker/campaign/real ALT, default transfer noise is empty, and protocol destinations are forbidden by the spec.
- **Suggested fix:** Rewrite docs and `info` from implemented behavior and explicitly label unavailable milestones. Remove claims of prevention/normalization and document sponsor tracing, timing, shared-router filtering, and atomic decoy failure.

### I12. On-chain behavior has no runtime test or green build gate

- **Severity:** Important
- **Scenario:** 7, plus wrong-API/build confidence.
- **File:line evidence:** `programs/supersonic-tx/tests/router_tests.rs:1-51`; `programs/supersonic-tx/src/lib.rs:129-180`; repository lacks `.github/workflows/ci.yml`
- **Why it fails in prod:** Tests construct event structs and assert local booleans; they never execute `noop_decoy`, zero-count rejection, CPI account forwarding, or event logs in `solana-program-test`. WDAC prevented even host compilation, and Anchor could not run.
- **Suggested fix:** Add real program-runtime tests and CI for `cargo test --workspace` plus `anchor build` on Linux. Do not deploy or claim bar C until both are green.

### I13. Public custom decoy hooks bypass fail-soft policy

- **Severity:** Important
- **Scenario:** 8, 10 — composable misuse and arbitrary program instructions.
- **File:line evidence:** `crates/supersonic-tx-sdk/src/builder.rs:79-83`, `103-107`; `crates/supersonic-tx-sdk/src/noise.rs:10-18`; `crates/supersonic-tx-core/src/types.rs:40-44`
- **Why it fails in prod:** Any `DecoyGenerator` can emit arbitrary instructions/program IDs, and `DecoyKind::Custom` presents arbitrary program payloads as supported. Builder validation does not inspect them for signer requirements, executable destinations, duplicate compute-budget instructions, or failure risk, so a “decoy” can abort the real transfer.
- **Suggested fix:** Separate an explicitly unsafe/custom API from the fail-soft builder. Validate supported instruction classes and reject unknown custom generators in atomic cast mode.

## Minor findings

### M1. Invalid security levels silently become Standard

- **Severity:** Minor
- **Scenario:** 1, 3 — operator typo.
- **File:line evidence:** `crates/supersonic-tx-cli/src/main.rs:75-81`
- **Why it fails in prod:** Values such as `--level paranoidd` do not error; they silently select Standard. Operators can run a materially different profile than requested.
- **Suggested fix:** Use a Clap `ValueEnum` over `ObfuscationLevel` and reject unknown values.

### M2. The public Benford sampler can loop forever

- **Severity:** Minor
- **Scenario:** 10 — SDK misuse.
- **File:line evidence:** `crates/supersonic-tx-sdk/src/noise.rs:198-218`
- **Why it fails in prod:** The rejection loop has no validation or attempt bound. Reversed or unreachable ranges never terminate.
- **Suggested fix:** Return `Result<u64, InvalidDecoyConfig>`, validate supported bounds first, and cap attempts.

### M3. Custom compute limits can overflow before clamping

- **Severity:** Minor
- **Scenario:** 10 — composable SDK misuse.
- **File:line evidence:** `crates/supersonic-tx-sdk/src/noise.rs:269-307`
- **Why it fails in prod:** `(base_limit * jitter_percent)` and `base_limit + jitter` use `u32` arithmetic before `.min(1_400_000)`. A large public `custom_limit_base` can panic in checked builds or wrap in release.
- **Suggested fix:** Validate/clamp the base first or use saturating/wider arithmetic before converting to `u32`.

### M4. Hardcoded program IDs use panic/fabrication fallbacks

- **Severity:** Minor
- **Scenario:** 7, 9; panic/`expect` scan.
- **File:line evidence:** `crates/supersonic-tx-core/src/lib.rs:11-13`; `crates/supersonic-tx-sdk/src/noise.rs:114-128`, `282-287`, `366-368`; `crates/supersonic-tx-sdk/src/builder.rs:214-244`
- **Why it fails in prod:** Several invariant pubkeys are parsed with `unwrap`/`expect`, while `AnchorRouterNoise::default` uniquely hides an invalid core ID by inventing `Pubkey::new_unique()`. A constant regression should fail deterministically, not panic deep in a cast or target a random program.
- **Suggested fix:** Use compile-time/static pubkey constants where possible and one startup validation returning a typed error. Remove the random fallback.

### M5. Windows drive-relative secret paths pass the relative-path gate

- **Severity:** Minor
- **Scenario:** 5, security/Windows paths.
- **File:line evidence:** `crates/account-cooker/src/types.rs:98-121` at HEAD (same logic in observed worktree `146-169`)
- **Why it fails in prod:** The validator rejects `C:\key.json` but accepts drive-relative `C:key.json`. Windows drive-relative resolution depends on process state and can escape the intended handoff directory semantics; no loader exists yet to contain it.
- **Suggested fix:** Reject any leading drive prefix (`^[A-Za-z]:`) and Windows device/ADS syntax, then canonicalize the joined parent and verify containment when loading.

## Scenario conclusions

1. **Devnet first cast:** offline assembly only; no RPC simulation, signing, balance check, or deploy check. All levels emit router noops; Standard/Paranoid do not emit promised transfers without explicit sinks.
2. **`--send`:** safely refused; no internal broadcast path exists. The deprecated SDK wrapper still creates default-signature transactions as a downstream footgun.
3. **Missing/underfunded/bad RPC/empty sinks:** missing keypair becomes an unfunded ephemeral payer; underfunding and RPC are never checked; default empty transfer sinks are silently omitted.
4. **ALT:** synthetic/invented, never fetched.
5. **Cooker handoff:** schema v1/version/path validation exists; load, fund, resolve, cast, permissions, and required-secret validation do not.
6. **Campaign:** absent.
7. **Program:** `noop_decoy` is simple but runtime-unverified; CPI can silently skip and emit success; count is not honest.
8. **MTU:** target instructions are not removed by `shrink_decoys`, which is good. Final diagnostics can be stale, and CU-limit retention is not guaranteed.
9. **Program ID:** source/config strings match; build, deployment, key custody, and cluster availability are unproven.
10. **Composable API:** sink omission is silent, custom generators bypass fail-soft constraints, and an unsigned transaction wrapper remains public.

## Executive summary — top 5 blockers before devnet with real SOL

1. **Implement real signer resolution, RPC blockhash/balance checks, simulation, and gated send; stop calling offline estimation simulation.**
2. **Remove synthetic ALT construction and fetch/verify actual on-chain lookup tables.**
3. **Deploy and verify the router or disable router noops by default; current atomic casts depend on an unproven program account.**
4. **Fix `execute_fuzzy_bundle` so target CPI cannot be silently skipped and events/counts prove what executed.**
5. **Finish account-cooker + campaign isolation and obtain green Linux CI (`cargo test --workspace` and `anchor build`) before using real funds.**

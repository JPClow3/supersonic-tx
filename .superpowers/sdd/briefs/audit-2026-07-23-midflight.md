# Midflight audit — `supersonic-tx` bar C

Date: 2026-07-23  
Audit basis: current `feature/bar-c` git history, tracked source, authoritative spec/plan, progress ledger, and Task 1–5 briefs/reports/review packages.

## Executive verdict

**Overall health: MIXED.**

Tasks 1–5 are implemented and committed at source level. The important Task 5 sink-validation finding is materially fixed, and the current CLI cannot broadcast placeholder-signature transactions. However, the bar-C product is not close to shippable yet: Tasks 6–20 remain incomplete, the live CLI still fabricates ALT contents, signing/cooker/campaign/CI/deploy work is absent, and no Rust or Anchor runtime gate has passed on this host.

Count: **5 of 20 plan tasks implemented/committed; 15 incomplete.** This is not a bar-C completion claim.

## 1. Progress ledger versus git reality

### Repository state

- Branch: `feature/bar-c`
- HEAD: `3e003ec71ea454d6e8d8bc94e059291ef05e453d`
- No upstream branch is shown by `git branch -vv`.
- Worktree is not clean. The ledger, most briefs/reports/review packages, `README.md`, `ARCHITECTURE.md`, and `docs/superpowers/` are untracked.
- Only Task 4 and Task 5 reports are tracked under `.superpowers/sdd/briefs/`.
- The progress ledger itself has never been committed.

### Commit list, oldest first

1. `0cc235571cea181caca98fd2ae0f737846e1d2ef` — `chore: fix MIT typo and quarantine agent notes`
2. `6b7e922708f03188901deacbc0f29b94c6f598ab` — `build: add workspace bincode for SDK and CLI`
3. `67fed6c08d63bfe7543461e1185b500a62e16fdf` — `feat: replace placeholder program id with generated key pubkey`
4. `6776cefccb83209af85c40ab726c7b06bfe14e3f` — `fix: compile V0 via v0::Message::try_compile and separate size estimate`
5. `608c45945aee981577f4781b4fd1df01a9978018` — `fix: refuse CLI broadcast of unsigned placeholder txs`
6. `3bb36a2434e0fb7c189e919269088f1f92a60aed` — `fix: use fail-soft tip/sink decoys instead of fake DEX destinations`
7. `3e003ec71ea454d6e8d8bc94e059291ef05e453d` — `fix: validate fail-soft decoy sinks and deny program IDs`

### Task reconciliation

| Plan task | Ledger claim | Git/source finding | Audit status |
| --- | --- | --- | --- |
| 1 | Complete at `0cc2355` | License, ignore rules, and archive note are tracked | Complete |
| 2 | Complete through `6b7e922` | Workspace `bincode` wiring is tracked; full compile was not proven | Complete with verification gap |
| 3 | Complete through `67fed6c` | Program ID is synchronized in program/core/Anchor config; no keypair is tracked | Complete with verification/deploy gap |
| 4 | Complete through `608c459` | Correct V0 compile path and CLI unsigned-send refusal are present | Complete with verification gap |
| 5 | Not recorded | Two commits, `3bb36a2` and `3e003ec`, implement and fix Task 5 | Complete with verification gap |
| 6–20 | No claim | Required commits/features are absent; a few early artifacts exist | Incomplete |

The two Task 5 commits are not reflected in the ledger. The ledger header also still says the branch is “to be created,” although it exists.

Partial work must not be counted as task completion:

- Task 6: shrink-priority implementation exists from Task 4, but the Task 6 priority tests and commit are absent.
- Task 18: `Cargo.lock` is tracked, but `.github/workflows/ci.yml` is absent and the required gates are not green.
- Tasks 7–17 and 19–20 lack their defining deliverables.

## 2. Binding-constraint audit

| Constraint | Source evidence | Result |
| --- | --- | --- |
| No live `Super111…` program ID | Runtime constants use `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`; `Super111…` appears only in the core negative test | Held |
| No `VersionedMessage::try_compile` call | No such call exists; the text appears only in a test name/report context | Held |
| V0 compile path | `builder.rs` calls `v0::Message::try_compile` and wraps with `VersionedMessage::V0` | Held |
| No CLI placeholder broadcast | No `send_transaction` call exists in Rust source; CLI uses unsigned messages only for assembly/size diagnostics | Held |
| `--send` refusal before signing | `cast --send` returns `Unsupported` before any broadcast | Held |
| Fake Jupiter key absent from product behavior | Exact key occurs only in a negative sink-rejection test; runtime deny logic uses a `JUP6` prefix | Held, test-only occurrence |
| Fail-soft sink validation | Private destinations, `DecoySink::cooked`, `from_cooked_sinks`, and known-program deny-list are present; invalid builder input emits no statistical transfers | Held, with limitations |
| Deploy keypair not committed | `git ls-tree`/`git ls-files` show no `target/deploy/*-keypair.json`; `/target/` and the explicit keypair pattern are ignored | Held |
| `.agents/` ignored | `.gitignore` contains `.agents/`; git reports the local directory as ignored and no `.agents` path is tracked | Held |

Important qualifications:

- `Signature::default()` still exists in `estimate_tx_size` and the deprecated compatibility wrapper. Neither has a current send path, but the wrapper should eventually be removed or tightly contained when signing lands.
- The current CLI still creates a fake in-memory `AddressLookupTableAccount` from payer/target addresses. This violates spec §7.2, but belongs to unfinished Tasks 13/15. It is not currently broadcast because `--send` refuses.
- The plan numbers signing as **Task 14**, not Task 12. Task 12 is deploy documentation. Current source/report messages saying signing arrives in “Task 12” are stale.

## 3. Task 5 fix completeness

The Important finding is substantially resolved:

- `StatisticalTransferNoise` has private destinations.
- There is no public `StatisticalTransferNoise::new(Vec<Pubkey>)` or `from_sinks(Vec<Pubkey>)` escape hatch.
- `DecoySink::cooked` rejects the System, SPL Token, Associated Token, Compute Budget, and Memo program IDs plus known Jupiter/Raydium/Orca-style prefixes.
- `from_cooked_sinks` validates all supplied keys before constructing the generator.
- Empty defaults generate no transfer decoys.
- Builder defaults no longer include fake DEX destinations.
- Tests are present for fake-default absence, injected sinks, known-program rejection, successful cooked-wallet transfers, and router-noop counts.

Limitations:

- “Cooked” provenance is naming/documentation, not type-enforced ownership proof: `from_cooked_sinks` still accepts raw `Vec<Pubkey>`.
- A static deny-list cannot detect every executable account. The source correctly acknowledges that RPC validation is still required.
- `FuzzyBundleBuilder::with_sinks` silently discards the entire statistical generator if any sink is invalid. This is safe/fail-soft, but poor operator diagnostics.
- The tests exist only as source evidence; they did not execute successfully on this host.

## 4. Environment and verification gaps

Current direct check:

- Explicit Cargo binary: `cargo 1.97.1`.
- `cargo test -p supersonic-tx-sdk -- --nocapture` failed before project compilation.
- WDAC blocked `target\debug\build\proc-macro2-...\build-script-build` with Windows error 4551.
- `cargo` and `anchor` are not available on the normal PowerShell PATH; `anchor` was not runnable.

Therefore the following have **not** been runtime-proven:

- Any current core, SDK, CLI, or workspace unit test.
- Task 5 regression tests.
- Rust compilation/type correctness of the current HEAD.
- `cargo test --workspace`.
- `anchor build`.
- On-chain router behavior, CPI behavior, or `decoy_count == 0` rejection under `solana-program-test`.
- CLI `--send` refusal through an executed binary.
- V0 serialization/MTU behavior under the pinned Solana dependencies.
- Simulation, RPC, ALT, signing, deploy, or devnet smoke behavior.
- Formatting of all current Rust changes.

Static checks did establish the git/source facts above and `git diff --check` reported no whitespace errors. The existing router tests are only struct/boolean assertions, not program-runtime tests.

## 5. Remaining work, Tasks 6–20

6. Add explicit MTU shrink-priority tests and prove CU retention.
7. Create `account-cooker` schema-v1 crate/types and serde round-trip.
8. Implement cooker key generation, secure key-file layout, handoff write/load/resolve.
9. Implement funding, draining, underfund refusal, and reuse warnings.
10. Add CLI `cook`.
11. Replace weak router tests with real `solana-program-test` coverage.
12. Add truthful devnet deploy documentation and verify commands.
13. Add real RPC ALT fetch/deserialization and fallback.
14. Add complete signing plus simulate/send gating; reject missing/default signatures.
15. Rewire CLI cast/simulate to handoff, signing, real blockhash, real ALT, and honest simulation.
16. Add campaign planner with real-intent isolation.
17. Add CLI campaign with default isolation and best-effort decoy semantics.
18. Add workspace/Anchor CI and obtain green gates; lockfile is already tracked.
19. Rewrite README/architecture/info output with an honest threat model and no fake-protocol narrative.
20. Run the end-to-end bar-C checklist and fix discovered gaps.

Primary blocker: WDAC prevents local Rust build-script execution. A clean Linux/CI runner or a host policy exception is needed for credible verification. Feature work is also still substantial; cooker, signing, ALT, campaign, program tests, CI, and deploy path are absent.

## 6. Risk register

### Critical/high

1. **No trustworthy green gate.** Current HEAD has not compiled or tested; bar C explicitly forbids completion claims without `cargo test --workspace` and `anchor build`.
2. **Live fake ALT construction remains.** The CLI fabricates ALT addresses instead of fetching on-chain data. It must be removed before signed sending is enabled.
3. **Signing/send path is missing.** Current refusal is safe, but the product cannot perform its core cast operation.
4. **Account cooker and campaign are missing.** These are central bar-C differentiators, not optional polish.
5. **Program behavior is untested.** Existing tests do not execute the Anchor program or CPI path.
6. **No CI or deploy proof.** There is no workflow, deploy guide, recorded deployment, or smoke result.

### Medium

7. **Documentation honesty risk.** Untracked `ARCHITECTURE.md` still recommends Jupiter/Raydium/Orca protocol-account decoys. CLI `info` claims the engine is “ACTIVE,” and simulation output claims “HIGH” mitigation despite no runtime proof.
8. **Source-of-truth risk.** The authoritative spec/plan, ledger, and most reports are untracked; the ledger is stale and omits Task 5.
9. **Sink assurance is incomplete.** Static deny-listing cannot establish non-executable ownership; cooker/RPC integration must supply stronger validation.
10. **Silent sink rejection.** Builder fail-soft behavior hides invalid configuration, which can mislead users about statistical decoy coverage.
11. **Program-ID operability.** Constants are synchronized, but no deployment/build proof establishes that the corresponding ignored keypair remains available and usable.

## Recommended next action

Update and commit the progress ledger/audit evidence without claiming green tests, then perform Task 6’s focused tests. In parallel operational terms, establish a Linux/CI verification path immediately; otherwise every later implementation task will accumulate uncompiled risk.

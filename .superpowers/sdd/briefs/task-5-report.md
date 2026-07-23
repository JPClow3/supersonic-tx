# Task 5 Report

Status: Implemented on `feature/bar-c`.

Changes:
- Replaced fake Jupiter/Raydium/Orca statistical destinations with `default_tip_allowlist()` (empty by default).
- Added `from_sinks`, `with_tips`, and fail-soft empty-sink generation.
- Added `FuzzyBundleBuilder::with_sinks`.
- Fixed `AnchorRouterNoise` counts to Light 1, Standard 1, Paranoid 2.
- Exported `MemoNoise`.
- Added the specified regression tests.

Tests:
- RED command attempted: `cargo test -p supersonic-tx-sdk statistical_noise_rejects_fake_jupiter_default router_noop_counts_match_spec_standard -- --nocapture`
- Blocked before compilation: `cargo` and `rustfmt` are unavailable on PATH.
- `git diff --check`: passed.
- Fake Jupiter key grep: no match in SDK source.

Concerns:
- Full Rust test suite and formatting could not run until the Rust toolchain is available.
- Existing unrelated untracked workspace files were not staged.

## Review-fix evidence

- Removed public `StatisticalTransferNoise::new` and `from_sinks` escape hatches.
- Added private destinations plus `DecoySink::cooked` / `from_cooked_sinks`, returning an error for denied program addresses.
- Hard-denied System, SPL Token, Associated Token, Compute Budget, Memo, and known Jupiter/Raydium/Orca-style program prefixes. Full executable detection still requires RPC.
- Updated builder sink injection to fail soft when validation rejects a sink.
- Added regression tests for System/Token/fake-Jupiter rejection and successful cooked-wallet transfers.
- `git diff --check`: passed.
- `cargo test -p supersonic-tx-sdk` and `cargo fmt --all -- --check`: unavailable because `cargo` is not installed on PATH.

## Task 5 re-review fixes

- Removed public arbitrary-pubkey `DecoySink::cooked` and `from_cooked_sinks`.
- Added opaque `TrustedSystemAccount` provenance handoff with deny-list enforcement,
  allowlist-only tip construction, and explicit RPC-validation limitations.
- Changed statistical noise and builder sink injection to accept validated sink
  types; empty sink injection now returns `InvalidDecoyConfig`.
- Added `SinkValidationMode`; on-chain mode fails clearly until an RPC checker is wired.
- Added regression coverage for program-like destinations and trusted sink transfers.
- Verification: `git diff --check` passed; Cargo/rustfmt remain unavailable on PATH.

## Critical Task 5 sealing evidence

- Removed the public arbitrary-pubkey `TrustedSystemAccount::assume_system_wallet`
  constructor; the remaining test-only helper is `#[cfg(test)]` and private.
- Added `TrustedSystemAccount::from_cooker_decoy_sink(&CookedAccount)`, which
  accepts only `CookedRole::DecoySink`, parses the handoff pubkey, and applies
  the existing program deny-list. `FeePayer` and `DrainTarget` are rejected.
- Tip construction remains allowlist-gated, and `with_tips` can only receive
  opaque accounts minted through the tip/cooker paths.
- Added regression tests for valid cooker sinks and rejected cooker roles.
- Verification: `git diff --check` passed; `cargo` and `rustfmt` are unavailable
  on PATH in this environment.

# Task 4 Report — Fix V0 message compile and unsigned size-check path

**Status:** DONE_WITH_CONCERNS
**Branch:** `feature/bar-c`
**Base:** `67fed6c`
**Commit:** `6776cef` — `fix: compile V0 via v0::Message::try_compile and separate size estimate`
**Date:** 2026-07-23

## Summary

Updated `FuzzyBundleBuilder` to compile Solana 1.18 V0 messages through
`v0::Message::try_compile` and `VersionedMessage::V0(...)`. Added a separate
`estimate_tx_size` helper that serializes placeholder signature slots only for
size estimation, and changed the shrink loop to return an unsigned
`VersionedMessage`.

Decoy shrinking now removes statistical system transfers first, then memo
instructions, then router noops while preserving one router noop, and finally
removes other decoys without removing the last compute-budget instruction.
Execution order is re-shuffled after every removal.

## TDD

1. Replaced the builder compilation test with
   `compiles_v0_message_without_versioned_message_try_compile`.
2. Ran the required red command. The host did not reach Rust compilation because
   Cargo was not on the PowerShell PATH (`cargo` was not recognized).
3. Implemented the minimal V0 compile, estimation, message-building, and shrink
   APIs.
4. Ran the full SDK test command using the discovered Cargo executable:

   ```text
   cargo test -p supersonic-tx-sdk -- --nocapture
   ```

   Verification was blocked by Windows App Control / WDAC os error 4551 while
   executing the `proc-macro2` build script.

## Verification and self-review

- `VersionedMessage::try_compile` removed from product source.
- `v0::Message::try_compile` followed by `VersionedMessage::V0(...)` is used.
- `MAX_TX_PAYLOAD_BYTES` remains the exact `1232`-byte limit.
- Placeholder signatures are confined to `estimate_tx_size` and the deprecated
  compatibility wrapper, which explicitly warns that its output is unsigned
  and must not be submitted.
- The MTU overflow test now exercises `build_versioned_message`.
- `rustfmt` completed successfully for `builder.rs`.
- `git diff --check` completed without whitespace errors.
- No keypairs, secrets, or `.agents/` files were staged.

## Commit

The required commit is:

```text
fix: compile V0 via v0::Message::try_compile and separate size estimate
```

## Concerns

1. Full Cargo tests could not complete on this host because WDAC blocks freshly
   built Cargo build scripts with os error 4551.
2. The deprecated `build_versioned_transaction` wrapper remains for existing
   CLI call sites, but it returns placeholder signatures for estimation only;
   Task 12 must provide real signing before broadcast.

## Files changed

- `crates/supersonic-tx-sdk/src/builder.rs`
- `crates/supersonic-tx-core/src/types.rs` — no changes were needed; existing
  `SupersonicError` variants cover this task.

## Critical finding fix — CLI unsigned broadcast guard

- Replaced every CLI use of the deprecated `build_versioned_transaction` with
  `build_versioned_message`; size diagnostics continue through the SDK's
  estimation-only `estimate_tx_size` helper.
- Removed the CLI `send_transaction` path. `cast` is now always a dry run by
  default, and `cast --send` exits clearly with an unsupported error stating
  that Task 12 `sign_versioned_tx` is required. No unsigned or
  `Signature::default()` transaction can be broadcast by the CLI.
- Updated CLI tests to cover unsigned message building and the `--send` flag.
- Grep verification: no `build_versioned_transaction`, `send_transaction`, or
  `Signature::default` references remain under `crates/supersonic-tx-cli`.
- `git diff --check` passed. Cargo/rustfmt verification was blocked because
  `cargo` and `rustfmt` are not available on this PowerShell PATH; the prior
  SDK verification was blocked by WDAC os error 4551 as recorded above.

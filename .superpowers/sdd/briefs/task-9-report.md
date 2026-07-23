# Task 9 Report

## Status

Implemented fund/drain APIs, underfund refusal, reuse warnings, and `CookerError`
in the account-cooker crate. Unit tests were added before implementation.

## Verification

- RED test command could not run: `cargo` is unavailable in the environment.
- GREEN/full test command could not run for the same reason.
- `git diff --check` passed.

## Notes

Funding and draining use the nonblocking `RpcClient`; draining reserves the RPC
fee before transferring the available balance. Reuse detection checks duplicate
pubkeys and matching keypairs under `out_dir/keys`.

## Review follow-up

- Drain now skips `DrainTarget` accounts before resolving keypair paths, so a
  valid pathless target does not fail with `MissingSecretKeyPath`.
- Drain fee messages use the fetched recent blockhash and unavailable fee
  estimates return an error instead of permitting a full-balance transfer.
- Drain amount math preserves the zero-data rent-exempt minimum.
- Added pure tests for rent-preserving drain math and pathless target skipping.
- Verification: `cargo test -p account-cooker` could not run because `cargo`
  is unavailable in the environment; `git diff --check` was run before commit.

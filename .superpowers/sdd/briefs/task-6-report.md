# Task 6 Report — MTU shrink-priority unit tests

Date: 2026-07-23
Branch: `feature/bar-c`

## Result

Implemented the Task 6 scope in `crates/supersonic-tx-sdk/src/builder.rs`:

- Exposed `shrink_decoys` as `pub(crate)`.
- Added the test-only `shrink_decoys_for_test` wrapper.
- Added `shrink_drops_statistical_before_memo`, proving a statistical
  system-transfer decoy is removed before memo padding while the compute-unit
  instruction remains.
- Preserved the existing shrink order: statistical transfer → memo → extra
  router noop → other removable decoy while retaining a compute-budget
  instruction when present.

## Verification

- `git diff --check`: passed.
- Focused Cargo test was attempted but could not start because `cargo` is not
  available on the local PowerShell PATH. Earlier audit evidence also records
  WDAC blocking Rust build-script execution on this host.
- No `cargo test --workspace` or `anchor build` result is claimed.

## Commit

The implementation is committed as required by the plan.

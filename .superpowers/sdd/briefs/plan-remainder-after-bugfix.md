# Plan remainder after production bugfix

The source-level bugfixes for Tasks 12–17 and the honest documentation work are
already present. Do not re-implement them.

## Priority 1 — obtain release-gate evidence

Run on Linux (or use the repository CI) and retain the results:

```text
cargo test --workspace --locked
anchor build
```

Resolve any failures found by those commands. The current Windows host cannot
complete this verification because WDAC blocks generated Rust build scripts and
`anchor` is unavailable.

## Priority 2 — record deployment and smoke evidence

Using the existing `docs/deploy.md` path, deploy `supersonic_tx` to the intended
devnet cluster, verify the recorded program ID and executable loader ownership,
then run the documented cook → simulate → cast `--send` smoke flow. Record the
cluster, program ID, transaction/signature evidence, and date in the product
deployment documentation. Do not use real SOL before this evidence exists.

## Priority 3 — close Task 20

Re-run the complete Task 20 checklist after the Linux build/test and deployment
gates pass, including forbidden-pattern checks and presence/wiring checks for
the existing ALT, signing, campaign, cook, CI, and deploy artifacts. Update the
ledger with command results. No empty “verification” commit is needed if no
source changes are required.

Plan-numbering note: the checked plan calls CI “Task 18” and honest docs “Task
19”; some controller notes refer to those two items in the opposite order.

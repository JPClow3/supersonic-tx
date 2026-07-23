# supersonic-tx

Rust tooling for behavioral obscurity on Solana. It interleaves a real instruction with
fail-soft compute-budget, memo, validated wallet-transfer, and optional deployed-router
noise. It is not a mixer, anonymity system, or protection against a determined analyst.

## Safety status

- `simulate` and the default `cast` path sign a V0 transaction and call RPC
  `simulateTransaction`; they fail on bad RPC, missing keys, insufficient balance, or
  instruction errors.
- Broadcast occurs only with `cast --send` or `campaign --send`, after successful
  simulation and only with non-default signatures.
- `--alt` fetches and validates the real lookup-table account. Failure falls back to a
  non-ALT V0 message and MTU shrink.
- Router noise is off by default. `--via-router` first verifies that the configured
  program account exists and is executable.
- Transfer sinks are accepted only after RPC proves they are non-executable,
  system-owned accounts.

No deployment is claimed in this repository. Follow [docs/deploy.md](docs/deploy.md),
record the cluster deployment, and pass CI before enabling router noise with real SOL.

## Workspace

- `crates/account-cooker`: key generation, sponsor funding, drain, and schema-v1 handoff
- `crates/supersonic-tx-core`: shared types, errors, program ID, and 1232-byte limit
- `crates/supersonic-tx-sdk`: builder, ALT resolver, signing/RPC, and campaign planner
- `crates/supersonic-tx-cli`: `cook`, `simulate`, `cast`, `campaign`, and `info`
- `programs/supersonic-tx`: Anchor noop and opt-in CPI router

Pinned compatibility: Solana `~1.18`, Anchor `0.30.1`, MIT.

## CLI

```text
supersonic-tx cook --sponsor-keypair sponsor.json --out-dir cooked
supersonic-tx simulate --target <PUBKEY> --handoff cooked/handoff-<time>.json
supersonic-tx cast --target <PUBKEY> --handoff <PATH> [--alt <ALT>] [--send]
supersonic-tx campaign --target <PUBKEY> --handoff <PATH> --txs 2 [--send]
```

`campaign` isolates the real intent by default. Decoy-only transaction failures are
best-effort; a real-intent simulation or send failure is fatal.

## Threat model

This may add noise against basic wallet-graph and transaction-shape heuristics. It does
not hide sponsor-to-cooker funding, timing, account locks, target instruction data,
shared-router use, or activity from human review. Atomic `cast` decoys share the fate of
the real intent; use isolated campaign mode when that risk is unacceptable.

## Deployments

None recorded. Operator steps: [docs/deploy.md](docs/deploy.md).

## Verification

```text
cargo test --workspace --locked
anchor build
```

The GitHub Actions workflow is the Linux build gate. A local environment blocked by
Windows Application Control is not evidence of a passing release.

Licensed under [MIT](LICENSE-MIT).

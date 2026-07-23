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
  program account exists and is executable, then routes the target transfer through CPI.
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
supersonic-tx assemble [--payer <PUBKEY>] [--target <PUBKEY>]
supersonic-tx simulate --target <PUBKEY> --handoff cooked/handoff-<time>.json
supersonic-tx cast --target <PUBKEY> --handoff <PATH> [--alt <ALT>] [--send]
supersonic-tx campaign --target <PUBKEY> --handoff <PATH> --txs 2 [--send]
```

`assemble` is the unsigned, keyless diagnostics path; `simulate` is signed RPC
simulation. `campaign` prebuilds every transaction through MTU shrink, reserves the
real-intent spend and fee before each decoy, and supports `--drain-to <PUBKEY>` only
with `--send --handoff`. Decoy-only failures are best-effort; real-intent failures are
fatal.

`cook` refuses to overwrite any existing deterministic key path. Use a fresh output
directory for every cook. On Windows, create a private directory before cooking:

```powershell
mkdir cooked
icacls cooked /inheritance:r /grant:r "$env:USERNAME:(OI)(CI)F"
```

Review the resulting ACL with `icacls cooked`; never fund keys in a broadly accessible
directory and never commit cooked key files.

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

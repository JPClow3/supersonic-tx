# supersonic-tx

Rust toolkit for **behavioral obscurity** on Solana: interleave a real instruction with
fail-soft compute-budget, memo, RPC-validated transfer sinks, and optional deployed-router
noise. Not a mixer, anonymity system, or protection against a determined analyst.

Design spec: [docs/superpowers/specs/2026-07-23-supersonic-tx-design.md](docs/superpowers/specs/2026-07-23-supersonic-tx-design.md)

## What it does / does not do

| Helps against (partially) | Does **not** stop |
| --- | --- |
| Naive wallet-graph clustering | Sponsor ? cooker funding trace |
| Simple CU / shape heuristics | Timing correlation across campaign txs |
| Single-obvious-instruction filters | Human review, CEX attribution |
| | Shared router program-id filtering |
| | Unique target instruction data |

Atomic `cast` decoys share the fate of the real intent. Use `campaign` with default
`--isolate-intent` when decoy failure must not abort the action.

## Install and build

**Toolchain:** Solana ~1.18, Anchor 0.30.1, Rust stable. Primary development and CI run
on **Linux (Docker recommended on Windows).**

### Native / CLI tests (full workspace)

```bash
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target rust:latest \
  bash -c 'rustup component add rustfmt && cargo fmt --all -- --check && cargo test --workspace --locked'

docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target/program-tests \
  -w /workspace/programs/supersonic-tx-tests -e CARGO_TARGET_DIR=/workspace-target/program-tests \
  rust:latest cargo test --locked
```

**56 tests** (50 workspace + 6 router crate). Log: `.superpowers/sdd/briefs/bar-c-cargo-test-2026-07-23.log`.

Build the CLI:

```bash
cargo build --release -p supersonic-tx-cli --locked
# binary: target/release/supersonic-tx
```

### SBF program + IDL (dual-lock path)

Full `anchor build` on the **full workspace** can fail under Anchor 0.30.1 / Cargo 1.79
(edition2024 transitive deps). Use the **dual-lock** SBF path instead:

| Use | Lock file | Image |
| --- | --- | --- |
| Native / `cargo test --workspace --locked` | Root `Cargo.lock` | `rust:latest` |
| SBF / `cargo build-sbf` / `anchor idl build` | `.superpowers/sdd/briefs/Cargo.lock.sbf.v3` | `backpackapp/build:v0.30.1` |

```bash
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target \
  backpackapp/build:v0.30.1 \
  bash .superpowers/sdd/briefs/bar-c-build-sbf-only.sh
```

Produces `target/deploy/supersonic_tx.so` and `target/idl/supersonic_tx.json`. Details:
[docs/deploy.md](docs/deploy.md), [ARCHITECTURE.md](ARCHITECTURE.md).

## Workspace

| Crate / program | Role |
| --- | --- |
| `crates/account-cooker` | Key generation, sponsor funding, drain, schema-v1 handoff |
| `crates/supersonic-tx-core` | Types, errors, program ID, 1232-byte limit |
| `crates/supersonic-tx-sdk` | Builder, ALT resolver, signing/RPC, campaign planner |
| `crates/supersonic-tx-cli` | `cook`, `simulate`, `cast`, `campaign`, `info` |
| `programs/supersonic-tx` | Anchor noop + opt-in CPI router |

`programs/supersonic-tx-tests/` is a **standalone** workspace (own `Cargo.lock`).

## CLI

Default RPC: `https://api.devnet.solana.com`. Provide exactly one of `--keypair` or
`--handoff` for signed paths. Broadcast requires explicit `--send`.

```text
supersonic-tx cook --sponsor-keypair sponsor.json --out-dir cooked/ \
  [--rpc-url URL] [--cluster devnet|localnet|mainnet-beta] \
  [--sinks 2] [--fee-payer-lamports 50000000] [--sink-lamports 2000000] [--dry-run]

supersonic-tx assemble [--level light|standard|paranoid] \
  [--payer PUBKEY] [--target PUBKEY] [--amount LAMPORTS]

supersonic-tx simulate --target PUBKEY [--amount LAMPORTS] \
  [--level standard] [--rpc-url URL] (--keypair PATH | --handoff PATH) \
  [--alt ALT_PUBKEY] [--tip PUBKEY ...] [--via-router]

supersonic-tx cast --target PUBKEY [--amount LAMPORTS] [--level standard] \
  [--rpc-url URL] (--keypair PATH | --handoff PATH) \
  [--alt ALT_PUBKEY] [--tip PUBKEY ...] [--via-router] [--send]

supersonic-tx campaign --target PUBKEY [--amount LAMPORTS] [--level standard] \
  [--rpc-url URL] (--keypair PATH | --handoff PATH) \
  [--txs 2] [--isolate-intent true] [--alt ALT_PUBKEY] [--tip PUBKEY ...] \
  [--send] [--drain-to PUBKEY]

supersonic-tx info
```

- **`cook`** — writes keypairs + `handoff-<ts>.json`; refuses to overwrite existing key
  paths. Use a fresh `--out-dir` each run. `--dry-run` skips funding.
- **`assemble`** — unsigned offline diagnostics (no RPC, no keys).
- **`simulate`** — sign + `simulateTransaction`; never broadcasts.
- **`cast`** — one atomic fuzzy tx; `--via-router` checks program is executable first.
- **`campaign`** — multi-tx plan; decoy failures are best-effort; real-intent failure is
  fatal. `--drain-to` requires `--send --handoff`.

Transfer noise needs RPC-validated sinks: cooked `DecoySink` accounts from `--handoff`, or
`--tip` pubkeys on an allowlist. Without sinks, transfer noise is disabled.

On Windows, restrict the cook output directory before funding keys:

```powershell
mkdir cooked
icacls cooked /inheritance:r /grant:r "$env:USERNAME:(OI)(CI)F"
```

Never commit cooked keys or deploy keypairs.

## Deployments

| Cluster | Program ID | Recorded (UTC) | Notes |
| --- | --- | --- | --- |
| localnet | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` | 2026-07-23 | Smoke PASS — Docker validator + `cook` ? `cast --via-router --send`. Genesis hash is ephemeral per `--reset`. |
| devnet | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` | — | **Blocked:** program not on-chain; deployer needs funded wallet (public faucet returned HTTP 429 at closeout). |

Program pubkey is fixed in `declare_id!`, `PROGRAM_ID_STR`, and `Anchor.toml`. **Public-cluster
deploy and live smoke are blocked without a funded deployer keypair** (not stored in this repo).

Operator steps: [docs/deploy.md](docs/deploy.md). Smoke checklist: [docs/smoke.md](docs/smoke.md).
Closeout evidence: `.superpowers/sdd/briefs/bar-c-closeout-2026-07-23.md`.

## Safety behavior

- `simulate` and default `cast`/`campaign` sign a V0 tx and call `simulateTransaction`.
- `--send` (or `campaign --send`) broadcasts only after successful simulation and non-default
  signatures.
- `--alt` fetches the real lookup-table account; failure falls back to non-ALT V0 + MTU shrink.
- `--via-router` is off by default; enables CPI wrapper only after executable-program check.
- Sinks must be non-executable, system-owned accounts (provenance via `TrustedSystemAccount`).

## Further reading

- [ARCHITECTURE.md](ARCHITECTURE.md) — router, sinks, campaign isolation, dual-lock builds
- [docs/deploy.md](docs/deploy.md) — localnet and devnet deploy
- [docs/smoke.md](docs/smoke.md) — post-deploy smoke commands

Licensed under [MIT](LICENSE-MIT).

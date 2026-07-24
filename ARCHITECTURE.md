# Architecture

**Approach 1 (v1):** one global shared Anchor router (`programs/supersonic-tx`) plus an
off-chain orchestrator (SDK + CLI). The router supplies opt-in noop/CPI decoys; entropy,
campaign scheduling, ALT selection, and MTU shrink live off-chain.

```text
account-cooker  →  handoff JSON  →  CLI  →  SDK (builder / campaign / ALT / sign)
                                              ↓
                                   programs/supersonic-tx (shared router, optional CPI)
                                              ↓
                                         Solana RPC
```

There is **no Jupiter or DEX decoy narrative**. SOL statistical transfer noise goes only to
RPC-validated system-wallet sinks (cooked `DecoySink` accounts or explicit `--tip` allowlist).
SPL Token / Token-2022 decoys are an optional SDK additive (`with_token_routes`); they do not
replace SOL sink policy and are not yet wired through the CLI or cooker.

## Components

| Layer | Responsibility |
| --- | --- |
| **account-cooker** | Fresh keypairs; sponsor-funded fee payer + sinks; schema-v1 handoff; optional drain |
| **supersonic-tx-core** | `ObfuscationLevel`, manifest types, typed RPC + domain errors, `MAX_TX_PAYLOAD_BYTES = 1232`, program ID |
| **supersonic-tx-sdk** | Decoy generators (incl. `TokenTransferNoise`), `FuzzyBundleBuilder`, `AltResolver`, `CampaignPlanner`, `classify_client_error` / sign/simulate/send |
| **supersonic-tx-cli** | Operator surface: `cook`, `assemble`, `simulate`, `cast`, `campaign`, `info` |
| **supersonic_tx (Anchor)** | `noop_decoy`; `execute_fuzzy_bundle` CPI wrapper (opt-in via `--via-router`) |

Program ID: `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` (synced across `declare_id!`,
core constant, deploy keypair, `Anchor.toml`).

## Fail-soft sinks and provenance

Allowed atomic decoys:

- Compute-budget instructions
- Memo program instructions
- System transfers to **validated sinks** only
- Optional SPL Token / Token-2022 `TransferChecked` decoys (`TokenDecoyRoute` —
  caller-supplied funded token accounts; no RPC mint/ATA check in-SDK yet)
- Router `noop_decoy` (after deployment check)
- Target instruction (user intent), optionally wrapped by router CPI

`TrustedSystemAccount` records provenance before RPC validation:

- **`from_cooker_decoy_sink`** — handoff `DecoySink` role + secret path under cook dir
- **`try_from_tip_allowlist`** — operator-supplied `--tip` pubkey on the CLI allowlist

`DecoySink::validate_on_chain` rejects executable accounts and non-system owners. Without
validated SOL sinks, the builder calls `without_transfer_noise()` (CU/memo/router only).
`with_token_routes` is additive and does **not** satisfy the SOL sink gate.

Security levels gate decoy counts; arbitrary custom generators and transfers to unknown
program IDs are not exposed on the safe builder path.

## ALT and MTU

`AltResolver` RPC-fetches the lookup table; it is never synthesized in memory. On fetch/decode
failure, compilation falls back to non-ALT V0 and shrinks decoys. Shrink order drops
statistical System **or** token transfers first, then memo, priority price, and extra router
noise—never target instructions. The builder returns manifest, `VersionedMessage`, and
serialized size together.

## Signing and RPC

`sign_versioned_tx` requires all signers; default signatures are rejected before simulate or
send. `simulate_and_send` always runs `simulateTransaction`; broadcast uses
`send_and_confirm_transaction` so multi-tx campaigns and `--drain-to` observe finalized
balances. Broadcast is gated by `SendOptions { broadcast: true }` (`--send`).

RPC / client failures map through `classify_client_error` into typed
`SupersonicError` variants (`RpcBlockhashNotFound`, `RpcInsufficientFundsForFee`,
`RpcAlreadyProcessed`, `RpcAccountInUse`, `RpcTransport`, else `RpcError`).
`is_transient_rpc()` is true for blockhash-not-found, account-in-use, and transport —
CLI `cast` rebuilds once with a fresh blockhash on those. Insufficient funds and
already-processed are **not** treated as transient.

Cluster is checked via genesis hash (`cook` / handoff load). `localnet` additionally requires
a loopback `--rpc-url` (`localhost` / `127.0.0.1`). `--alt` simulation failures on
cast/campaign trigger a non-ALT retry path.

## Campaign isolation

`CampaignPlanner` labels each planned tx `DecoyOnly` or `RealIntent`. With
`--isolate-intent` (default **true**), statistical transfers stay out of the real-intent tx.

- Decoy-only simulate/send errors: logged, execution continues (best-effort).
- Real-intent errors: fatal.
- CLI prebuilds all txs, computes live fees and transfer spend, skips decoys that would breach
  the real-intent lamport reserve, then recompiles each tx with a fresh blockhash before send.
- `--drain-to` (requires `--send --handoff`) runs only after the real intent confirms.

## Router (opt-in)

Default SDK/CLI path uses a direct System Program transfer for the target. `--via-router`:

1. `verify_executable_program` on the deployed router
2. Wrap target ix in `execute_fuzzy_bundle` with `routed_instruction_count: 1`
3. Router CPI executes the wrapped instruction; emits `BundleExecuted`

Default decoy path uses `noop_decoy` only—not the CPI wrapper. Router noise is separately
opt-in and never assumed deployed.

## Build and test layout

| Gate | Command / path | Lock |
| --- | --- | --- |
| Format + unit tests | `cargo test --workspace --locked` (Docker `rust:latest`) | Root `Cargo.lock` |
| Router integration | `cargo test --locked` in `programs/supersonic-tx-tests/` (`solana-program-test`) | Standalone lock |
| SBF artifact | `bar-c-build-sbf-only.sh` in `backpackapp/build:v0.30.1` | `Cargo.lock.sbf.v3` |
| IDL | `anchor idl build -p supersonic_tx` (same script tail) | SBF lock |
| Full `anchor build` (all members) | CI job runs it; local full-workspace metadata may fail on Cargo 1.79 | Root lock |

Router program-test coverage (`programs/supersonic-tx-tests/tests/router_tests.rs`):

- `noop_decoy` succeeds
- `execute_fuzzy_bundle` rejects `routed_instruction_count` 0 and ≠1
- Missing / non-executable CPI target → `MissingCpiProgram`
- Successful System Program transfer CPI via remaining accounts
- Failed CPI `invoke` → `CpiExecutionFailed` (no success event without CPI)

Dual-lock policy avoids edition2024 transitive pulls during `cargo build-sbf` metadata.
See [docs/deploy.md](docs/deploy.md) and `.superpowers/sdd/briefs/bar-c-status-2026-07-23.md`.

## Open follow-ups

Tracked in README **Roadmap**: campaign transient-RPC retry parity; CLI/cooker surfaces for
token decoys; mint/ATA RPC validation. bankrun is not in use.

## Threat model (honest)

Behavioral obscurity against automated graph/shape heuristics—not cryptographic privacy.
Sponsor funding, timing, account locks, instruction data, and shared-router use remain
visible. See README and `supersonic-tx info`.

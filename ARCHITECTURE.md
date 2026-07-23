# Architecture

`account-cooker` produces funded keypairs and a schema-v1 handoff. The CLI resolves
those keys, verifies transfer sinks through RPC, and asks the SDK to build one final
manifest. That exact manifest is compiled into a Solana V0 message, checked against the
1232-byte payload limit, signed, simulated, and optionally sent.

```text
account-cooker -> handoff -> CLI -> SDK builder/ALT/signing -> Solana RPC
                                      |
                                      +-> optional deployed Anchor router
```

## Decoy policy

Allowed atomic decoys are compute-budget instructions, memos, transfers to
RPC-validated non-executable system wallets, and router noops after deployment
verification. Arbitrary custom generators and transfers to program IDs are not exposed
by the safe builder.

Security levels require validated transfer sinks unless the caller explicitly chooses
`without_transfer_noise`. Router noise is separately opt-in; no default transaction
depends on an unproven deployment.

## ALT and MTU

An ALT pubkey is resolved from its on-chain account and owner, never synthesized.
Compilation falls back to no ALT when fetch or decode fails. Shrink order removes
transfer noise, memo noise, priority price, and extra router noise before the protected
compute-unit limit. Target instructions are never removed.

The builder returns the final `BundleManifest`, `VersionedMessage`, and serialized size
together so diagnostics cannot describe a different randomized bundle.

## Signing and RPC

`VersionedTransaction::try_new` must resolve every signer. Default signatures are
rejected before simulation or broadcast. `simulate_and_send` always runs
`simulateTransaction`; sending is conditional on an explicit broadcast option.

## Campaign isolation

`CampaignPlanner` labels transactions as `DecoyOnly` or `RealIntent`. With isolation
enabled (the CLI default), statistical transfers never enter the real-intent
transaction. Decoy-only errors are logged and execution continues; real-intent errors
fail the command. Every planned manifest uses the shared MTU shrink loop. The CLI
prebuilds the campaign, computes live fees and System Program transfer spend, and skips
any decoy that would breach the real-intent reserve.

## Router

The default SDK path does not include the router. `--via-router` wraps the target
instruction itself; router noops are not a substitute for routing. The CPI wrapper accepts exactly one
routed CPI, rejects a missing or non-executable target, invokes it, and emits success
only afterward. Its event reports the executed routed-instruction count rather than a
caller-asserted decoy count.

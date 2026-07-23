# Full bug sweep fix report — 2026-07-23

Branch: `feature/bar-c`

Verification: `cargo fmt --all -- --check`, `cargo metadata --no-deps`, and
`git diff --check` pass. `cargo check --workspace` and tests are blocked before
project compilation by Windows Application Control error 4551 while launching
the generated `serde` build script. Anchor is not installed and no remote,
deployment, CI result, or smoke signature is available.

| ID | Status | Fix | Commit(s) |
| --- | --- | --- | --- |
| C1 | FIXED | Key writes use create-new semantics, preserve metadata, roll back partial writes, and test byte preservation on a second cook. | `8c8bbe3` |
| I1 | FIXED | Campaigns prebuild all messages, calculate live fees/System transfers, reserve the real intent, and skip reserve-breaching decoys. | `9e4099a`, `b3fe184` |
| I2 | FIXED | Planned manifests compile through the shared MTU shrink loop; CLI reports final sizes. | `9e4099a`, `b3fe184` |
| I3 | FIXED | Key resolution is account-indexed and skips pathless drain targets. | `8c8bbe3`, `b3fe184` |
| I4 | FIXED | `--via-router` now wraps the target transfer in the router CPI instruction after executable-program verification. | `b3fe184` |
| I5 | FIXED | Router argument/docs/logs now describe exactly one routed instruction, not fictitious decoys. | `b3fe184` |
| I6 | FIXED | Cook and handoff consumption compare RPC genesis hash with declared cluster. | `b3fe184` |
| I7 | FIXED | Funding rejects cooker/handoff/signer sponsor mismatch before any RPC call. | `8c8bbe3` |
| I8 | FIXED | Key writing validates source metadata and preserves funding/minimum fields. | `8c8bbe3` |
| I9 | FIXED | Campaign supports explicit `--drain-to` only after `--send` with a handoff and reports drain failure separately. | `b3fe184` |
| I10 | FIXED | Added keyless unsigned `assemble`; retained `simulate` as truthful signed RPC simulation with diagnostics split documented. | `b3fe184`, `0ac4902` |
| I11 | FIXED | Documented private Windows directory/ACL setup, ACL review, non-overwrite behavior, and key custody. | `0ac4902` |
| I12 | DEFERRED | Linux CI, Anchor build, deployment, and smoke evidence require a remote/tooling/cluster not present in this workspace. No release claim is made. | — |
| I13 | FIXED | ALT resolver rejects deactivated/current-slot-extended tables; cast and campaign retry failed ALT transactions without ALT. | `9e4099a`, `b3fe184` |
| I14 | FIXED | Authoritative spec, plan, and sweep are tracked. | `0ac4902` |

Additional constraint checks: broadcasts still pass through fully-signed
transactions; `Signature::default()` remains size-estimation-only; transfer
sinks now require tip-allowlist or cooker provenance before RPC validation.

Totals: Critical 1 FIXED / 0 remaining. Important 13 FIXED / 0 source
remaining / 1 external release gate DEFERRED.

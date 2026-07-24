# Docs update — 2026-07-23

Branch: `feature/bar-c`

## Summary

Rewrote user-facing documentation to match bar-C closeout evidence (localnet smoke PASS,
devnet deploy blocked, dual-lock SBF build path). Removed contradictory “no deployment
claimed” language; kept honest threat-model limits aligned with the design spec.

## Files changed

| File | Change |
| --- | --- |
| `README.md` | Install/build (Docker + dual-lock), full CLI flags, deployments table, threat table, links |
| `ARCHITECTURE.md` | Approach 1 diagram, TrustedSystemAccount sinks, campaign/signing/ALT, dual-lock test matrix |
| `docs/deploy.md` | Localnet validated path, dual-lock commands, devnet funded-wallet caveat, keypair warning |
| `docs/smoke.md` | **New** — operator smoke copied/cleaned from `bar-c-smoke-2026-07-23.md` |

## Not changed

- Design spec (`docs/superpowers/specs/2026-07-23-supersonic-tx-design.md`) — threat model honesty preserved
- Application code, keypairs, CI workflow
- Internal briefs under `.superpowers/sdd/briefs/` (referenced, not duplicated)

## Operator highlights

- Program ID: `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`
- Localnet: deploy + `cast --via-router --send` validated 2026-07-23
- Devnet: **blocked without funded deployer** (faucet 429 at closeout)
- SBF: use `bar-c-build-sbf-only.sh` + `Cargo.lock.sbf.v3`, not full-workspace `anchor build` alone
- Tests: 56 (50 workspace + 6 router crate) via Docker `rust:latest`

## Cross-links added

- README → ARCHITECTURE, deploy.md, smoke.md, closeout brief
- deploy.md → smoke.md, ARCHITECTURE dual-lock section
- smoke.md → deploy.md, bar-c-smoke brief

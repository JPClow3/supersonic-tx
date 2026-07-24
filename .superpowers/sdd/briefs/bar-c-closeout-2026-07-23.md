# Bar C closeout — 2026-07-23

Branch: `feature/bar-c` | HEAD: `5658e31`

## Phase A — Anchor build + IDL

**Verdict: PASS (dual-lock path; full `anchor build` on full workspace still fails)**

| Check | Result | Evidence |
| --- | --- | --- |
| Toolchain | OK | Docker `backpackapp/build:v0.30.1` — solana 1.18.17, anchor 0.30.1, cargo 1.79 |
| Product workspace members | OK | Root keeps program + account-cooker + core + sdk + cli; `programs/supersonic-tx-tests` excluded |
| SBF artifact | OK | `supersonic_tx.so` **196640** bytes (`target/deploy/`, volume `/workspace-target/deploy/`) |
| IDL | OK | `target/idl/supersonic_tx.json` **3670** bytes via `anchor idl build -p supersonic_tx` |
| `cargo metadata --locked` (slim + `Cargo.lock.sbf.v3`) | OK | `METADATA_OK` in closeout run |
| `cargo build-sbf … -- --locked` | OK | exit 0 — script `.superpowers/sdd/briefs/bar-c-build-sbf-only.sh` |
| Monolithic `anchor build` (full members) | FAIL | `cargo_build_sbf` metadata pulls `block-buffer` 0.12.1 / edition2024 — log `bar-c-anchor-build-closeout-2026-07-23.log` |

Dual-lock policy: root `Cargo.lock` (native/CI) + `.superpowers/sdd/briefs/Cargo.lock.sbf.v3` (SBF). Slim members only for SBF/IDL steps.

## Phase B — Devnet deploy

**Verdict: FAIL (blocked — no funded deployer)**

| Check | Result | Evidence |
| --- | --- | --- |
| Program pubkey (keypair) | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` | matches `declare_id!`, `PROGRAM_ID_STR`, `Anchor.toml` |
| On-chain program | **Not deployed** | `solana program show GVWC…` → account not found (devnet) |
| Deployer wallet | Ephemeral `%TEMP%\\supersonic-bar-c-closeout\\deployer.json` (not committed) | `solana balance` 0 SOL |
| Airdrop | FAIL | CLI + JSON-RPC `requestAirdrop` → rate limit / HTTP 429 |
| Deploy signature | — | not obtained |
| README Deployments | Unchanged | still "None recorded" until on-chain deploy |

No keypairs committed.

## Phase C — Live smoke (cook → cast --send)

**Verdict: FAIL (depends on B + SOL)**

No devnet transactions broadcast. See `bar-c-smoke-2026-07-23.md`.

## Commits (branch tip)

- `5658e31` docs(bar-c): document dual lock for SBF vs native workspace
- `b694a48` chore(bar-c): restore workspace, re-run Linux tests, document Anchor gap

No new commits in this closeout pass.

## Blockers

1. Devnet deploy/smoke pending funded deployer (public faucet 429).
2. Full `anchor build` on full workspace until SBF lock pins block edition2024 transitives for `cargo_build_sbf` metadata.

## Re-run (localnet)

1. `docker run -d --name supersonic-localnet -p 8899:8899 -p 8900:8900 backpackapp/build:v0.30.1 solana-test-validator --reset --quiet`
2. Deploy + smoke per `bar-c-smoke-2026-07-23.md` (Docker host network + Linux CLI binary).

## Artifacts

- `.superpowers/sdd/briefs/bar-c-build-sbf-only.sh`
- `.superpowers/sdd/briefs/bar-c-anchor-build-closeout-2026-07-23.log`
- `.superpowers/sdd/briefs/bar-c-smoke-2026-07-23.md`


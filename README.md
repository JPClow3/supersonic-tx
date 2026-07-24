# supersonic-tx

**Rust toolkit that wraps a real Solana transfer in realistic decoy noise** (Benford amounts,
fail-soft CU/memo, cooker-funded sinks, optional shared Anchor router) for algotraders,
whales, and agents who want behavioral obscurity — **not** a mixer, anonymity set, shielded
pool, or ZK system.

**Judge entrypoint:** branch [`feature/bar-c`](https://github.com/JPClow3/supersonic-tx/tree/feature/bar-c)
(repo default) — tag [`v0.1.0-bar-c`](https://github.com/JPClow3/supersonic-tx/releases/tag/v0.1.0-bar-c)
— MIT [`LICENSE-MIT`](LICENSE-MIT).

| Quick links | |
| --- | --- |
| Architecture | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Deploy | [docs/deploy.md](docs/deploy.md) |
| Smoke + sigs | [docs/smoke.md](docs/smoke.md) |
| Design | [docs/superpowers/specs/2026-07-23-supersonic-tx-design.md](docs/superpowers/specs/2026-07-23-supersonic-tx-design.md) |

---

## 60-second install

Toolchain: Solana ~1.18, Anchor 0.30.1, Rust stable. Prefer **Linux Docker** (native Windows
Cargo may hit WDAC 4551 on build scripts).

```bash
git clone https://github.com/JPClow3/supersonic-tx.git
cd supersonic-tx
git checkout v0.1.0-bar-c

# Format + workspace tests (CI mirrors this)
docker run --rm -v "$PWD:/workspace" -w /workspace rust:latest \
  bash -c 'rustup component add rustfmt && cargo fmt --all -- --check && cargo test --workspace --locked'

# CLI binary
cargo build --release -p supersonic-tx-cli --locked
# -> target/release/supersonic-tx
```

SBF `.so` + IDL (dual-lock; not full-workspace `anchor build`):

```bash
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target \
  backpackapp/build:v0.30.1 \
  bash .superpowers/sdd/briefs/bar-c-build-sbf-only.sh
```

---

## Demo: cook → cast (localnet)

Requires a running local validator + deployed program
`GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` ([docs/deploy.md](docs/deploy.md),
[docs/smoke.md](docs/smoke.md)).

```bash
export RPC=http://127.0.0.1:8899
export DEPLOYER=/path/to/deployer.json   # never commit

supersonic-tx cook --sponsor-keypair "$DEPLOYER" --out-dir /tmp/cooked \
  --rpc-url "$RPC" --cluster localnet

supersonic-tx cast --handoff /tmp/cooked/handoff-*.json \
  --target So11111111111111111111111111111111111111112 --amount 100000 \
  --rpc-url "$RPC" --via-router --send
```

**Expected shape:** simulation OK, payload under **1232** bytes (reference run: **484/1232**),
several decoys interleaved (reference: **6**), confirmed signature printed. Reference cast
(2026-07-24 localnet refresh at commit `971fb96`):
`39iuxT1gGq2jsRzUwtb5441ME6aebnTuXhSzrd3yfFvttpGW5LBqGKnbsxbu7FvaTJh3ztek5EWjFw7cQqG7tpdE`.

Levels: `light` | `standard` | `paranoid`. Campaign isolation: `campaign --isolate-intent true`
(default) keeps statistical transfers out of the real-intent tx. Opt-in router CPI:
`--via-router` on `simulate`/`cast`.

---

## Decoy kinds

| Kind | What it does |
| --- | --- |
| Statistical SOL transfers | Benford-ish amounts to cooker `DecoySink` accounts (TrustedSystemAccount + secret path) or allowlisted `--tip` |
| ComputeBudget | Fail-soft CU limit / price padding |
| Memo | Noise memos |
| Anchor router `noop_decoy` | Shared-program-id zero-op (`--via-router` path also offers `execute_fuzzy_bundle` CPI wrapper) |
| MTU shrink | Drop decoys in order until ≤1232 bytes; **never** drop the real intent |

Without RPC-validated sinks, the builder falls back to CU/memo/router-only (`without_transfer_noise`).

---

## Deployments

| Cluster | Program ID | Recorded (UTC) | Notes |
| --- | --- | --- | --- |
| localnet | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` | 2026-07-24 | Smoke **PASS** — Docker `supersonic-localnet` + `cook` → `cast --via-router --send`. Commit `971fb96`. Genesis `Dwas9mCe5QyEPZpJNXewNjhtYpHbcRK2vdN8zjUPfypi` (ephemeral per `--reset`). |
| devnet | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` | — | Not deployed — needs funded deployer wallet (public faucet may 429). Reproduce locally via [docs/deploy.md](docs/deploy.md) + [docs/smoke.md](docs/smoke.md). |

### Localnet signatures (judge copy-paste)

Localnet genesis is **ephemeral** — Solscan / Explorer links will **not** resolve these. Confirm with
`solana confirm <SIG> --url http://127.0.0.1:8899` against a live `solana-test-validator`, or re-run
the smoke path below.

| Step | Signature |
| --- | --- |
| Deployer airdrop | `3Rcnw2eq8cp9SgJYEYGhWHqN7SigZvzVyuAtug4T2iYePfzmTSU1U8mwnt6PLy1qxMTJnzeo6g2aLGJs5KsR2kRV` |
| Program deploy | `3ybMFoUh3oVDY51ZBJfj9ZUNjEiduArYbznNRNAm4cvyJFy16opcRkMpnt9QJwnN9dbwDWZV9LnbGfcGrnxgCCF1` |
| Cast `--via-router --send` | `39iuxT1gGq2jsRzUwtb5441ME6aebnTuXhSzrd3yfFvttpGW5LBqGKnbsxbu7FvaTJh3ztek5EWjFw7cQqG7tpdE` |

Cast shape: **484/1232** bytes, **6** decoys, status **Finalized**. Operator script:
`.superpowers/sdd/briefs/run-phase-b-smoke.sh` (after `docs/deploy.md` localnet validator + deploy).

---

## What this is / is not

| Claims | Non-claims |
| --- | --- |
| Rust E2E: cooker + CLI/SDK + Anchor router | Not a mixer / tumbler |
| Realistic noise (Benford, fail-soft sinks, MTU shrink, campaign isolate) | Not ZK / shielded / unlinkable funding |
| Composable `account-cooker` schema-v1 handoff | Not mainnet-ready ops by default |
| Shared router grows anonymity-set *fingerprint* with `PROGRAM_ID` | Not immune to analysts who know the program ID |

---

## Verification

| Check | Evidence |
| --- | --- |
| Workspace tests | `cargo test --workspace --locked` — GitHub Actions **rust** job ([workflow](.github/workflows/ci.yml)) |
| SBF `.so` + IDL | Dual-lock script in Actions **sbf** job (`backpackapp/build:v0.30.1`) |
| Localnet smoke | [docs/smoke.md](docs/smoke.md) — deploy + cast signatures refreshed 2026-07-24 (see **Deployments**) |
| Green CI run | [Actions run 30061171647](https://github.com/JPClow3/supersonic-tx/actions/runs/30061171647) — workspace `rust` + dual-lock `sbf` green |

---

## Threat model

Obscurity against **automated** graph/shape heuristics — not cryptographic privacy.

| Adversary / signal | Effect of this toolkit |
| --- | --- |
| Naive wallet-graph clustering | Partial — cooked sinks / tips add edges |
| Simple CU / shape heuristics | Partial — CU/memo/router padding |
| Single-obvious-instruction filters | Partial — interleaved decoys in same tx |
| Mempool / copy-trade bots | Weak — timing and unique ix data remain |
| Analyst who knows `PROGRAM_ID` | Filters on router easily |
| Sponsor → cooker funding trace | **Unaffected** (always visible) |
| Timing across campaign txs | **Unaffected** |
| CEX / KYC / human review | **Unaffected** |
| Unique target instruction data | **Unaffected** |

**Non-goals:** mixing, unlinkability of sponsor funding, ZK, ephemeral per-user programs as
default, Jito as a hard requirement, SPL decoy graphs beyond SOL system transfers.

Atomic `cast` decoys share fate with the real intent. Use `campaign` with default
`--isolate-intent true` when a decoy failure must not abort the action.

---

## System model

**Approach 1:** one global Anchor router + off-chain orchestrator.

```text
CLI (cook | simulate | cast | campaign | info)
  -> SDK (FuzzyBundleBuilder / CampaignPlanner / AltResolver / sign+RPC)
    -> core (types, errors, PROGRAM_ID, MAX_TX_PAYLOAD_BYTES=1232)
      -> optional CPI into programs/supersonic-tx
        -> Solana RPC
account-cooker --schema-v1 handoff--> CLI/SDK (fee payer + DecoySink secrets)
```

| Layer | On-chain | Off-chain |
| --- | --- | --- |
| Entropy, decoy counts, interleave order | - | SDK generators + builder |
| Campaign scheduling / isolate-intent | - | `CampaignPlanner` + CLI |
| ALT fetch, V0 compile, MTU shrink | - | `AltResolver` + builder |
| Sign / simulate / confirm-send | - | `sign_versioned_tx`, `simulate_and_send` |
| `noop_decoy`, `execute_fuzzy_bundle` | router program | SDK emits ixs only if opted in |
| System / Memo / ComputeBudget ixs | native programs | SDK builds them |

Default cast path uses a **direct System Program transfer** for the user intent. Router CPI
(`--via-router` on `simulate`/`cast`) is opt-in and increases shared-program-id fingerprint.

---

## Bundle pipeline

1. **Generators** (level-gated): `StatisticalTransferNoise`, `ComputeBudgetNoise`,
   `MemoNoise`, `AnchorRouterNoise` (`noop_decoy`). Arbitrary custom generators / transfers
   to program IDs are not on the safe builder path.
2. **Sink provenance** -- `TrustedSystemAccount` only:
   - `from_cooker_decoy_sink` -- handoff role `DecoySink` **and** non-empty `secret_key_path`
   - `try_from_tip_allowlist` -- `--tip` pubkey present in the CLI allowlist
3. **RPC validation** -- `DecoySink::validate_on_chain` rejects executable / non-system owners.
   Without validated sinks -> `without_transfer_noise()` (CU/memo/router only).
4. **MTU** -- serialize <= `MAX_TX_PAYLOAD_BYTES` (1232). Shrink drop order:
   statistical System transfers -> memo -> extra router noops (keep >=1) -> CU price -> other
   decoys. **Never** drop target instructions; protect at least one CU limit if present.
5. **V0 + ALT** -- `--alt` RPC-fetches the lookup table (never synthesized). Fetch/decode
   failure -> non-ALT V0 + shrink. Cast/campaign ALT sim failures retry without ALT.
6. **Sign / send** -- reject default signatures; always `simulateTransaction`; `--send`
   uses `send_and_confirm_transaction` so campaigns and `--drain-to` see confirmed balances.

---

## Program surface

Program ID: `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`
(synced: `declare_id!`, `PROGRAM_ID_STR`, `Anchor.toml`, deploy keypair).

| Instruction | Behavior |
| --- | --- |
| `noop_decoy(entropy_seed)` | Signer-gated zero-op; emits `DecoyExecuted` |
| `execute_fuzzy_bundle(bundle_seed, routed_instruction_count, instruction_data)` | Requires `routed_instruction_count == 1`; CPI to first remaining executable account; emits `BundleExecuted` **only after** successful `invoke` |

CPI honesty: missing/non-executable CPI program -> `MissingCpiProgram`; failed `invoke` ->
`CpiExecutionFailed` (no success event without CPI). Primary decoy path is `noop_decoy`;
CPI wrapper is cast/simulate `--via-router` only (after `verify_executable_program`).

---

## account-cooker

| Concern | Behavior |
| --- | --- |
| Schema | Handoff `schema_version: 1` only (`handoff-<unix_ts>.json`) |
| Roles | `FeePayer`, `DecoySink`, `DrainTarget` |
| Secrets | Paths under `--out-dir` (`keys/...`); never embed raw secrets in JSON |
| Fund | Sponsor System transfers; **fresh blockhash per confirm** (avoids mid-batch expiry) |
| Drain | Leaves rent-exempt minimum; CLI `--drain-to` needs `--send` + confirmed real intent |
| Overwrite | Refuses existing key files -- use a fresh `--out-dir` |
| Provenance | Sink usable as transfer noise only if role + `secret_key_path` pass cooker gate |

```json
{
  "schema_version": 1,
  "cluster": "localnet",
  "created_at_unix": 0,
  "sponsor_pubkey": "<base58>",
  "accounts": [
    {
      "role": "FeePayer",
      "pubkey": "<base58>",
      "secret_key_path": "keys/fee_payer.json",
      "funded_lamports": 50000000,
      "min_required_lamports": 10000000
    }
  ],
  "warnings": []
}
```

---

## Campaign

| Rule | Detail |
| --- | --- |
| Default | `--isolate-intent true` -- statistical transfers stay out of the real-intent tx |
| Kinds | `DecoyOnly` (best-effort) vs `RealIntent` (fatal on failure) |
| Preflight | Estimate fees/spend; skip decoys that would breach real-intent lamport reserve |
| Blockhash | Recompile + resign with a **fresh** blockhash immediately before each send |
| Drain | `--drain-to` requires `--send --handoff` and a confirmed real-intent broadcast |

`campaign` has no `--via-router` (direct System transfer for intent).

---

## Build / test / deploy

**Toolchain:** Solana ~1.18, Anchor 0.30.1, Rust stable. Prefer **Linux Docker** (native
Windows Cargo may hit WDAC error 4551 on build scripts).

### Workspace members

| Path | Role |
| --- | --- |
| `crates/account-cooker` | Keygen, fund, drain, handoff |
| `crates/supersonic-tx-core` | Types, errors, program ID, 1232 limit |
| `crates/supersonic-tx-sdk` | Builder, ALT, campaign, sign/RPC |
| `crates/supersonic-tx-cli` | Operator binary `supersonic-tx` |
| `programs/supersonic-tx` | Anchor router |
| `programs/supersonic-tx-tests/` | Standalone workspace (own lock); **excluded** from root |

### Dual-lock

| Use | Lock | Image |
| --- | --- | --- |
| Native / `cargo test --workspace --locked` | Root `Cargo.lock` | `rust:latest` |
| SBF / IDL | `.superpowers/sdd/briefs/Cargo.lock.sbf.v3` | `backpackapp/build:v0.30.1` |

Full `anchor build` on the **full** workspace can fail under Anchor 0.30.1 / Cargo 1.79
(edition2024 transitive deps). Use the dual-lock SBF script instead.

```bash
# Format + workspace tests
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target rust:latest \
  bash -c 'rustup component add rustfmt && cargo fmt --all -- --check && cargo test --workspace --locked'

# Router crate tests (standalone lock)
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target/program-tests \
  -w /workspace/programs/supersonic-tx-tests -e CARGO_TARGET_DIR=/workspace-target/program-tests \
  rust:latest cargo test --locked

# SBF .so + IDL
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target \
  backpackapp/build:v0.30.1 \
  bash .superpowers/sdd/briefs/bar-c-build-sbf-only.sh

# CLI binary
cargo build --release -p supersonic-tx-cli --locked
# -> target/release/supersonic-tx
```

Localnet smoke: [docs/smoke.md](docs/smoke.md). Deploy: [docs/deploy.md](docs/deploy.md).

---

## CLI reference

Default RPC: `https://api.devnet.solana.com`. Signed paths require exactly one of
`--keypair` or `--handoff`. Broadcast requires explicit `--send`.

```text
supersonic-tx cook \
  --sponsor-keypair PATH --out-dir DIR \
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
  [--txs 2] [--isolate-intent true|false] [--alt ALT_PUBKEY] [--tip PUBKEY ...] \
  [--send] [--drain-to PUBKEY]

supersonic-tx info
```

| Command | Notes |
| --- | --- |
| `cook` | Writes keypairs + handoff; refuses overwrite; `--dry-run` skips funding; cluster checked via genesis hash (`localnet` also requires loopback RPC) |
| `assemble` | Unsigned offline diagnostics -- no RPC, no keys |
| `simulate` | Sign + `simulateTransaction`; never broadcasts |
| `cast` | One atomic fuzzy V0 tx; `--via-router` checks executable program first |
| `campaign` | Multi-tx; decoy failures logged/continued; real-intent fatal; no `--via-router` |
| `info` | Prints identity / threat-model summary |

Windows cook dir (Unix uses mode `0600`; Windows inherits ACLs -- restrict before funding):

```powershell
mkdir cooked
icacls cooked /inheritance:r /grant:r "$env:USERNAME:(OI)(CI)F"
```

Never commit cooked keys, sponsor keys, or `target/deploy/*-keypair.json`.

---

## Limits / ops

| Item | Status |
| --- | --- |
| Dual-lock SBF vs native | Required for reproducible `.so`/IDL under Anchor 0.30.1 |
| Full-workspace `anchor build` | May fail on edition2024 metadata; use `bar-c-build-sbf-only.sh` |
| Devnet deploy/smoke | Needs **funded** deployer wallet (public faucet may 429) |
| Windows host Cargo | WDAC 4551 can block build scripts -- use Docker Linux |
| Shared `PROGRAM_ID` | Operable demo surface; also a clustering handle |

---

## Further reading

- [ARCHITECTURE.md](ARCHITECTURE.md) — sinks, campaign isolation, dual-lock detail
- [docs/deploy.md](docs/deploy.md) — localnet / devnet operator steps
- [docs/smoke.md](docs/smoke.md) — post-deploy checklist + reference signatures

Cluster status: see **Deployments** above. Licensed under [MIT](LICENSE-MIT).

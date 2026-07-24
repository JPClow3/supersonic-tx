# supersonic-tx Design Specification

| Field | Value |
| --- | --- |
| **Date** | 2026-07-23 |
| **Status** | Approved pending user file review |
| **Scope bar** | C (hardest) — production-ready fuzzy bundles + real account-cooker + multi-tx noise + ALT + CI + deploy path |
| **Bounty** | Superteam Brazil Rust-only Solana privacy (behavioral obscurity via fuzzy tx bundles) |
| **Primary path** | `supersonic-tx/` (this document lives under the deliverable project) |

---

## 1. Problem / mission

Solana transactions are fully public: accounts locked, instruction data, CU budgets, and transfer edges are permanent and machine-readable. Analytics, copy-traders, and MEV bots cluster wallets and fingerprint intent from those signals.

**Mission:** ship a Rust-only toolkit that makes *behavioral* intent harder to isolate by wrapping real actions in **plausible, on-chain-successful decoy instructions and multi-transaction campaigns**, without claiming cryptographic mixing, ZK privacy, or fund concealment.

This is **obscurity against automated graph / pattern analysis**, not anonymity against a determined human adversary with off-chain correlation.

---

## 2. Goals & non-goals

### 2.1 Goals (v1 success criteria)

1. **Global shared Anchor router** (`programs/supersonic-tx`) deployed with a real program ID (not a placeholder), exposing `noop_decoy` and an optional CPI wrapper, with Anchor events.
2. **Off-chain orchestrator** (`supersonic-tx-sdk` + CLI) that builds, signs, simulates, and sends Versioned (V0) transactions with real ALT fetch and MTU enforcement (`≤ 1232` serialized bytes).
3. **Real `account-cooker` crate** that generates fresh keypairs, funds them from a sponsor, optionally drains, and emits a versioned handoff JSON consumed by the CLI/SDK.
4. **Fail-soft decoys** that succeed on-chain using tip/fee sinks, self-transfers among cooked accounts, CU/memo noise, and router noops — never fake Jupiter IDs or transfers to program IDs.
5. **Campaign mode** that isolates the real intent into its own transaction when possible so decoy-tx failure does not abort the action.
6. **Honest docs**: threat model limits, MIT license typo fixed, `.agents/` quarantined from the product narrative.
7. **Ship path**: `Cargo.lock` committed, workspace CI (`cargo test --workspace` + `anchor build`), documented deploy steps for the router on a chosen cluster (devnet first, mainnet optional).

### 2.2 Non-goals (v1)

- Cryptographic mixers, shielded pools, or ZK proofs.
- Ephemeral / per-user deployable programs as the default path (reserved as future “paranoid” mode).
- Guaranteeing privacy against human investigators, CEX KYC linkage, or off-chain timing correlation.
- Jito bundles as a hard requirement (see §13 — stretch only).
- Full SPL token decoy *ops* beyond SOL system transfers (SDK `TokenDecoyRoute` /
  `TokenTransferNoise` landed post bar-C; CLI/cooker/mint validation remain open — see
  README **Roadmap**).
- Claiming “victory” or bounty completion without green `cargo test --workspace` and `anchor build`.

---

## 3. Chosen approach + alternatives rejected

### 3.1 Chosen: Approach 1 — Global shared Anchor router + off-chain orchestrator

```
account-cooker  →  handoff JSON  →  SDK builder / campaign planner  →  sign + send (+ ALT)
                                                                      ↓
                                                         programs/supersonic-tx (shared)
```

**Why:** one deployed program ID is operable for a bounty demo; off-chain code owns entropy, campaign scheduling, ALT selection, and MTU shrink. On-chain surface stays small and auditable.

### 3.2 Rejected alternatives

| Alternative | Why rejected for v1 |
| --- | --- |
| Ephemeral programs per cast | High deploy cost, key management, and CI complexity; useful as optional future “paranoid” mode only. |
| Pure off-chain decoys with no on-chain program | Weaker demo story for a Solana Rust bounty; loses router noop/events as a first-class decoy class. |
| On-chain-only interleaving with no cooker | Cannot supply unlinkable fee payers / self-transfer sinks; decoy edges collapse onto the user’s main wallet. |
| Fake protocol destinations (bogus Jupiter keys) | Fails realism policy; transfers to wrong/non-owned accounts or program IDs are detectable and often unsafe. |

---

## 4. Architecture

### 4.1 Workspace layout (target)

```
supersonic-tx/
├── Anchor.toml
├── Cargo.toml                 # workspace members listed below
├── Cargo.lock                 # committed
├── LICENSE-MIT
├── README.md                  # honest threat model; no agent victory claims
├── ARCHITECTURE.md            # aligned with this spec (or replaced by pointer)
├── .github/workflows/ci.yml
├── programs/
│   └── supersonic-tx/         # Anchor program crate name: supersonic_tx
├── crates/
│   ├── account-cooker/        # NEW — keygen, fund, drain, handoff
│   ├── supersonic-tx-core/    # types, levels, errors, program ID
│   ├── supersonic-tx-sdk/     # decoys, FuzzyBundleBuilder, ALT, campaign, signed V0
│   └── supersonic-tx-cli/     # cook / simulate / cast / campaign (+ info)
└── docs/superpowers/specs/
    └── 2026-07-23-supersonic-tx-design.md
```

**Workspace members (Cargo.toml):**

- `programs/supersonic-tx`
- `crates/account-cooker`
- `crates/supersonic-tx-core`
- `crates/supersonic-tx-sdk`
- `crates/supersonic-tx-cli`

### 4.2 Component responsibilities

| Component | Crate / path | Responsibility |
| --- | --- | --- |
| **account-cooker** | `crates/account-cooker` | Fresh `Keypair`s; sponsor-funded rent+fee balance; optional post-cast drain; `HandoffBundle` JSON; reuse warnings. |
| **supersonic-tx-core** | `crates/supersonic-tx-core` | `ObfuscationLevel`, `BundleManifest`, `SupersonicError`, `MAX_TX_PAYLOAD_BYTES = 1232`, real `PROGRAM_ID` / `PROGRAM_ID_STR`. |
| **supersonic-tx-sdk** | `crates/supersonic-tx-sdk` | `DecoyGenerator` implementations (`StatisticalTransferNoise`, `MemoNoise`, `ComputeBudgetNoise`, `AnchorRouterNoise`, optional `TokenTransferNoise` via `TokenDecoyRoute` / `TokenProgramKind` / `with_token_routes`), `FuzzyBundleBuilder`, real ALT RPC fetch, campaign planner, assemble → sign → simulate/send helpers. |
| **Anchor program** | `programs/supersonic-tx` | `noop_decoy`, optional `execute_fuzzy_bundle` CPI wrapper, events, reject `decoy_count == 0`. |
| **CLI** | `crates/supersonic-tx-cli` binary `supersonic-tx` | Subcommands: `cook`, `simulate`, `cast`, `campaign`, `info`. |

### 4.3 Key types (core + cooker)

```rust
// crates/supersonic-tx-core — canonical shared types
pub enum ObfuscationLevel { Light, Standard, Paranoid }

pub struct BundleManifest {
    pub target_instructions: Vec<Instruction>,
    pub decoy_instructions: Vec<Instruction>,
    pub level: ObfuscationLevel,
    pub execution_order: Vec<usize>,
}

pub const MAX_TX_PAYLOAD_BYTES: usize = 1232;
// PROGRAM_ID_STR set after `anchor keys` / deploy — never Super111...
```

```rust
// crates/supersonic-tx-sdk — live decoy surface (DecoyGenerator impls)
// Generators: StatisticalTransferNoise, MemoNoise, ComputeBudgetNoise,
//             AnchorRouterNoise, TokenTransferNoise
pub enum TokenProgramKind { SplToken, Token2022 }

pub struct TokenDecoyRoute {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub mint: Pubkey,
    pub decimals: u8,
    pub program: TokenProgramKind,
    pub min_amount: u64,
    pub max_amount: u64,
}
// FuzzyBundleBuilder::with_token_routes(routes) — additive; does not satisfy SOL sinks
```

```rust
// crates/account-cooker — handoff schema (serde)
pub struct HandoffBundle {
    pub schema_version: u32,          // = 1 for v1
    pub cluster: String,              // "devnet" | "mainnet-beta" | "localnet"
    pub created_at_unix: i64,
    pub sponsor_pubkey: String,       // base58
    pub accounts: Vec<CookedAccount>,
    pub warnings: Vec<String>,        // e.g. pubkey reuse notices
}

pub struct CookedAccount {
    pub role: CookedRole,             // FeePayer | DecoySink | DrainTarget
    pub pubkey: String,               // base58
    pub secret_key_path: Option<String>, // relative path under cooker out dir; never embed raw secret in JSON by default
    pub funded_lamports: u64,
    pub min_required_lamports: u64,
}

pub enum CookedRole { FeePayer, DecoySink, DrainTarget }
```

**Secret handling rule:** handoff JSON references keypair *files* written under a user-specified `--out-dir` (chmod-restricted on Unix; documented for Windows). Optional `--embed-secrets` is unsupported in v1 to reduce accidental leakage.

### 4.4 Handoff JSON schema (concrete)

File name convention: `handoff-<unix_ts>.json`.

```json
{
  "schema_version": 1,
  "cluster": "devnet",
  "created_at_unix": 1721750400,
  "sponsor_pubkey": "<base58>",
  "accounts": [
    {
      "role": "FeePayer",
      "pubkey": "<base58>",
      "secret_key_path": "keys/fee_payer.json",
      "funded_lamports": 50000000,
      "min_required_lamports": 10000000
    },
    {
      "role": "DecoySink",
      "pubkey": "<base58>",
      "secret_key_path": "keys/sink_0.json",
      "funded_lamports": 2000000,
      "min_required_lamports": 890880
    }
  ],
  "warnings": []
}
```

CLI/SDK load path: `--handoff <path>` resolves `secret_key_path` relative to the handoff file’s directory.

### 4.5 SDK public surface (v1)

| Type / fn | Role |
| --- | --- |
| `StatisticalTransferNoise` | Benford-ish lamports to **allowlisted sink/tip accounts** or cooked `DecoySink`s |
| `ComputeBudgetNoise` | `SetComputeUnitLimit` / optional price noise |
| `MemoNoise` | Memo program ix with short random memo |
| `AnchorRouterNoise` | `noop_decoy` against real program ID |
| `FuzzyBundleBuilder` | Targets + decoys → `BundleManifest` → ordered ixs |
| `AltResolver` | RPC-fetch ALT account data → `AddressLookupTableAccount` |
| `CampaignPlan` | Ordered list of `PlannedTx` (decoy-only vs real-intent) |
| `sign_versioned_tx` | Requires all required signers; **never** broadcasts unsigned / default signatures |
| `simulate_and_send` | `simulateTransaction` then send if simulation OK (cast); campaign may send decoys best-effort |

**API note:** compile V0 messages with the Solana 1.18-compatible path (`v0::Message::try_compile` / equivalent used by `solana_sdk ~1.18`), not a nonexistent `VersionedMessage::try_compile` helper if the installed SDK lacks it. Workspace pins `solana-sdk = "~1.18"` and declares `bincode` in workspace deps used by SDK + CLI.

---

## 5. Data flows

### 5.1 Single cast (one atomic transaction)

```
[cook] sponsor → fund FeePayer (+ DecoySinks)
         ↓
[handoff JSON]
         ↓
[cast] load handoff + keypairs
         ↓
  build target ix (user intent)
  generate fail-soft decoys (sinks / tips / CU / memo / router noop)
  interleave order
         ↓
  resolve ALT (RPC) or fallback non-ALT V0 + shrink decoys
         ↓
  compile V0 message → sign with required keypairs
         ↓
  simulateTransaction → on success, sendTransaction
         ↓
  on-chain: System / Memo / ComputeBudget / supersonic_tx router
```

**Atomicity:** one transaction — if any instruction fails, the whole tx fails. Therefore decoys in a single cast **must** be fail-soft (expected to succeed). Real intent shares fate with decoys in this mode; users who need isolation use `campaign`.

### 5.2 Campaign (multi-transaction noise)

```
[cook] → handoff
         ↓
[campaign] CampaignPlanner builds N transactions:
   Tx_1..Tx_k : decoy-only (tips, self-transfers, router noops, CU/memo)
   Tx_real    : real intent ± minimal fail-soft padding (optional Light decoys only)
   Tx_tail    : optional post-noise
         ↓
  For each decoy tx: sign → simulate → send (best-effort; log failures; continue)
  For Tx_real: sign → simulate → send (hard-fail on error)
  Optional: cooker drain of sinks back to sponsor / DrainTarget
```

**Isolation rule:** when `--isolate-intent` is set (default **true** for `campaign`), the real-intent transaction does not include statistical transfers that could fail; only CU/memo/router noops that are known-safe may pad it. Decoy txs never carry the real target instruction.

---

## 6. On-chain program design

### 6.1 Program identity

- Crate: `programs/supersonic-tx` (`supersonic_tx` module).
- Replace `declare_id!("Super111...")` and `supersonic_tx_core::PROGRAM_ID_STR` with the keypair under `target/deploy/supersonic_tx-keypair.json` (or Anchor-generated id) **before** any claimed deploy.
- `Anchor.toml` `[programs.devnet]` / `[programs.localnet]` must match.

### 6.2 Instructions

| Instruction | Accounts | Behavior | Success criteria |
| --- | --- | --- | --- |
| `noop_decoy(entropy_seed: u64)` | `authority: Signer` | `msg!`, emit `DecoyExecuted` | Always succeeds if signer present |
| `execute_fuzzy_bundle(bundle_seed, decoy_count, instruction_data)` | `authority`, `system_program`, remaining accounts | Reject if `decoy_count == 0`; optionally CPI to first remaining executable program with `instruction_data`; emit `BundleExecuted` | CPI path is **optional** in casts; primary decoy path is `noop_decoy` |

**v1 policy on CPI wrapper:** keep the instruction for bounty/demo and program tests, but the default SDK decoy path uses `noop_decoy` only. Casting arbitrary user CPIs through the router is opt-in (`--via-router`) and documented as increasing fingerprint surface (router program id appears).

### 6.3 Events & errors

- Events: `DecoyExecuted { authority, entropy_seed, timestamp }`, `BundleExecuted { authority, bundle_seed, decoy_count, timestamp }`.
- Errors: `InvalidBundleManifest` (`decoy_count == 0`), `CpiExecutionFailed`.

### 6.4 Program tests

Use `solana-program-test` (or Anchor test harness equivalent) in `programs/supersonic-tx/tests/`:

1. `noop_decoy` succeeds and emits event (or log contains expected marker).
2. `execute_fuzzy_bundle` with `decoy_count == 0` returns `InvalidBundleManifest`.
3. CPI happy path with a trivial executable stub / system transfer via remaining accounts (as applicable).

Inline unit tests that only assert `0 == 0` are insufficient and must be replaced or supplemented by program-test coverage.

---

## 7. Off-chain noise & realism

### 7.1 Fail-soft decoy catalog (allowed)

| Kind | Mechanism | Destination / target rules |
| --- | --- | --- |
| Tip / fee sink transfer | System transfer small Benford sample | Configured tip accounts (e.g. known public tip receivers) **or** cooked `DecoySink` pubkeys owned by handoff keypairs |
| Self-transfer | System transfer among cooked accounts | Only accounts whose secrets are in the handoff |
| Compute budget | `SetComputeUnitLimit` (± price) | Native compute budget program |
| Memo | Memo program ix | Short UTF-8 memo, length capped |
| Router noop | `noop_decoy` | Deployed `PROGRAM_ID` only |

### 7.2 Explicitly forbidden (v1)

- Invented / placeholder “Jupiter” (or any DEX) pubkeys that are not verified destinations.
- Transfers **to** program IDs (executable accounts) as if they were wallets.
- Decoys that require unsigned or partial signatures.
- Broadcast of `Signature::default()` placeholders.
- Fake in-memory ALT accounts that were never fetched from chain.

### 7.3 Amounts & levels

| Level | Statistical transfers | CU noise | Memo | Router noop |
| --- | --- | --- | --- | --- |
| Light | 1 | yes | optional | 0–1 |
| Standard | 3 | yes | yes | 1 |
| Paranoid | 5 | yes | yes | 1–2 |

Lamport samples stay in a documented band (default **1_000–50_000**) via Benford-leading-digit sampling already sketched in `StatisticalTransferNoise::sample_benford_lamports`.

### 7.4 ALT & MTU

1. If `--alt <pubkey>` provided: **RPC-fetch** ALT account, deserialize addresses, build `AddressLookupTableAccount`.
2. Compile V0 message; serialize with `bincode`; require `len <= 1232`.
3. On overflow: drop lowest-priority decoys (statistical transfers first, then memo, then extra noops; keep target ixs and at least one CU ix if present) and retry.
4. If ALT fetch fails: log warning, compile **non-ALT** V0, shrink decoys more aggressively, continue if MTU satisfied; else return `TransactionSizeExceeded`.

### 7.5 Signing & send

- Resolve all required signers from handoff + `--keypair` / payer flags.
- `VersionedTransaction::try_new(message, signers)` (or equivalent); abort if any signature missing.
- Never send unsigned transactions. Dry-run without key material stops at simulate/assembly and prints that broadcast was skipped.

---

## 8. account-cooker design

### 8.1 Library API

```text
Cooker::new(rpc, sponsor_keypair)
  .generate(n_sinks, fund_fee_payer_lamports, fund_sink_lamports)
  .write_keypair_dir(out_dir)
  .write_handoff(path) -> HandoffBundle
  .drain(handoff, destination) // optional
  .assert_funded_for_cast(handoff, estimated_fees) // refuse if underfunded
```

### 8.2 CLI mapping

Exposed both as:

- library used by `supersonic-tx cook`, and
- same logic callable from tests without RPC mocks inventing balances (integration tests may use `solana-test-validator` or documented devnet sponsor).

### 8.3 Policies

| Policy | Behavior |
| --- | --- |
| Underfunded cast | Cooker / cast **refuses** with a clear error listing shortfall vs `min_required_lamports` + estimated decoy+fee budget |
| Pubkey reuse | If a generated pubkey collides with an existing file in `--out-dir` or appears twice in one handoff, **warn** in `warnings[]` and CLI stderr; do not silently continue without surfacing it |
| Drain | Optional post-campaign: transfer remaining lamports from sinks/fee payer above rent-exempt minimum to `DrainTarget` or sponsor |

### 8.4 Current state

There is **no** `account-cooker` implementation in the tree today. v1 treats it as a first-class workspace crate, not a README fiction.

---

## 9. Error handling & security / threat model limits

### 9.1 Error handling matrix

| Situation | Behavior |
| --- | --- |
| Decoy would be unsafe / non-fail-soft | Do not emit; substitute allowed kind or shrink |
| Single-cast decoy would risk real intent | Prefer campaign isolation; single cast only uses fail-soft set |
| Campaign decoy tx fails | Log + continue; do not abort real intent |
| Real intent tx fails | Hard error; non-zero CLI exit |
| ALT unavailable | Fallback non-ALT + shrink |
| Missing signature | Abort before RPC send |
| Underfunded handoff | Refuse cast/campaign |
| `decoy_count == 0` on router CPI helper | On-chain reject |

### 9.2 Threat model (honest)

**Helps against (partially):** naive wallet clustering on transfer graphs; naive CU fingerprint filters; simple “single obvious ix” mempool heuristics.

**Does not stop:** adversaries who fund-trace the sponsor → cooker edge; timing correlation across campaign txs; human review; protocol-level unique instruction data; regulators/CEX attribution; anyone who knows the shared router program id and filters on it.

**Docs requirement:** README threat table must state these limits explicitly. Agent folders under `.agents/` must not be cited as proof of completion.

---

## 10. Testing strategy

| Layer | What | How |
| --- | --- | --- |
| Unit — noise | Benford sampler distribution bounds; generator counts per level | `crates/supersonic-tx-sdk` |
| Unit — builder | MTU shrink loop; ordered assembly; refuse unsigned send helpers | `supersonic-tx-sdk` |
| Unit — clap | Subcommand parsing for cook/simulate/cast/campaign/info | `supersonic-tx-cli` |
| Unit — cooker | Keygen uniqueness; handoff serde round-trip; reuse warning | `account-cooker` |
| Program | `noop_decoy`, CPI path, `decoy_count == 0` reject | `solana-program-test` |
| Integration | assemble → sign → `simulateTransaction` against local validator or devnet | `tests/` or CLI `--simulate-only` path with real RPC |

**CI gate:** `cargo test --workspace` and `anchor build` must both pass. Victory claims without these are invalid.

---

## 11. Ship checklist (CI, deploy, docs, license)

### 11.1 Repository hygiene

- [ ] Initialize git if absent; commit `Cargo.lock`.
- [ ] Fix `LICENSE-MIT` typo: `INCLUDINGRemind` → `INCLUDING`.
- [ ] Quarantine `.agents/`: add to `.gitignore` **or** move under `docs/internal/agents-archive/` with a README stating they are non-authoritative process notes — product docs must not depend on them.
- [ ] Align `ARCHITECTURE.md` / `README.md` with this spec (remove fake Jupiter decoy narrative; document fail-soft sinks + cooker).

### 11.2 CI (`.github/workflows/ci.yml`)

```yaml
# Conceptual jobs — implement equivalently
jobs:
  rust:
    runs-on: ubuntu-latest
    steps: [checkout, install Rust, cargo test --workspace]
  anchor:
    runs-on: ubuntu-latest
    steps: [checkout, install Solana + Anchor 0.30.1, anchor build]
```

### 11.3 Deploy path

1. `anchor keys list` / ensure program id consistency across `declare_id!`, core constant, `Anchor.toml`.
2. `anchor build && anchor deploy --provider.cluster devnet`.
3. Record deployed program id in README “Deployments” section.
4. Smoke: `supersonic-tx cook` → `simulate` → `cast` on devnet with funded sponsor.
5. Mainnet deploy is optional and must be an explicit operator decision (not required to call v1 “complete” for bounty demo if devnet + CI are green).

### 11.4 Dependency fixes (known)

- Add workspace `bincode` and wire through SDK/CLI `Cargo.toml`.
- Fix V0 compile API to match `solana-sdk ~1.18`.
- Remove default/dummy signatures from any broadcast path.

---

## 12. Current state / gaps

Snapshot of the tree as of this spec (must be closed during implementation):

| Gap | Location / symptom | Required fix |
| --- | --- | --- |
| Placeholder program ID `Super111...` | `programs/.../lib.rs`, `core::PROGRAM_ID_STR` | Real key + synced manifests |
| Unsigned / dummy signatures | `FuzzyBundleBuilder::build_versioned_transaction` fills `Signature::default()`; CLI notes dummy sigs | Sign before send; dry-run must not broadcast |
| Fake ALT | CLI builds `AddressLookupTableAccount` from payer/target without RPC fetch | `AltResolver` RPC path |
| Missing / fragile `bincode` + `try_compile` | SDK uses `bincode` / `VersionedMessage::try_compile` | Workspace dep + correct 1.18 API |
| Weak program tests | Inline asserts without runtime | `solana-program-test` coverage |
| Fake decoy destinations | `StatisticalTransferNoise::default_mainnet_destinations` bogus Jupiter key | Tip/sink allowlist + cooked sinks |
| Empty account-cooker | Not in workspace | New crate + CLI `cook` |
| No CI / Cargo.lock / git discipline | Repo hygiene | Lockfile + workflow |
| LICENSE typo | `INCLUDINGRemind` | Fix spelling |
| Agent victory claims without cargo | `.agents/**` auditor notes | Quarantine; CI is source of truth |

---

## 13. Out of scope / future

| Item | v1 stance | Status (post bar-C / `main`) |
| --- | --- | --- |
| Ephemeral per-cast programs | Future “paranoid” mode only | Unchanged |
| Jito bundled submission | **Stretch** — implement only if it plugs in cleanly behind a `Submitter` trait (`RpcSubmitter` default, `JitoSubmitter` optional feature). Not required for v1 done. | Unchanged |
| SPL token decoy edges | Future, fail-soft only | **Partial:** SDK routes + MTU shrink landed; CLI/cooker/RPC mint-ATA validation open |
| Typed RPC error variants + transient retry | Not in original v1 writeup | **Landed:** core variants + `classify_client_error`; `cast` retries on `is_transient_rpc` |
| Deeper router CPI program-tests | Spec required `solana-program-test` | **Landed** (incl. remaining-accounts CPI success + failed invoke); bankrun not used |
| Shared public tip registry service | Out of band; v1 uses config file / CLI flags + cooked sinks | Unchanged |
| Mobile / non-Rust clients | Out of scope | Unchanged |

---

## 14. Implementation phasing (ordered milestones)

### Milestone M0 — Spec lock & hygiene

- Land this design doc; fix LICENSE typo; decide `.agents/` quarantine approach; add `.gitignore` basics; ensure workspace builds after dep pins (`bincode`, Solana API).

**Exit:** docs + license + build graph sane; no product behavior claims yet.

### Milestone M1 — Core truthfulness

- Real program ID plumbing (local key even before deploy).
- Fix V0 message compile + MTU check without dummy broadcast.
- Replace fake decoy destinations with allowlisted tips + injectable sink list.
- Unit tests for Benford + MTU shrink.

**Exit:** `cargo test -p supersonic-tx-core -p supersonic-tx-sdk` green; no placeholder program id string in core.

### Milestone M2 — account-cooker

- Implement crate + handoff schema v1 + fund/drain/reuse warning + underfund refuse.
- CLI `cook` subcommand writing key dir + JSON.
- Unit tests for keygen/handoff.

**Exit:** cooker tests green; sample handoff validates against schema_version 1.

### Milestone M3 — Program tests + deployability

- `solana-program-test` for noop / CPI / decoy_count==0.
- `anchor build` clean; document deploy; optional devnet deploy.

**Exit:** program tests green; `anchor build` in CI.

### Milestone M4 — ALT + cast/simulate honesty

- Real ALT fetch + fallback shrink.
- CLI `simulate` / `cast` sign → simulate → send (send only with keys + explicit `--send`).
- Integration test: assemble → sign → simulateTransaction.

**Exit:** no fake ALT; no unsigned send; MTU enforced.

### Milestone M5 — Campaign + ship

- `CampaignPlan` + CLI `campaign` with isolate-intent default.
- CI workflow (`cargo test --workspace`, `anchor build`).
- README/ARCHITECTURE rewrite: honest threat model; Cargo.lock committed.
- Quarantine `.agents/`.

**Exit (bar C):** cooker + multi-tx campaign + ALT + CI + documented deploy path; stretch Jito left unimplemented or behind optional feature without blocking done.

---

## 15. CLI contract (normative)

Binary: `supersonic-tx`

| Subcommand | Required inputs | Behavior |
| --- | --- | --- |
| `cook` | `--sponsor-keypair`, `--out-dir`, `--rpc-url`, sink counts / fund amounts | Writes keys + `handoff-*.json` |
| `simulate` | `--level`, optional `--target`/`--amount`, optional `--handoff` | Build+sign-or-dry-run assemble; print decoy ratio, CU, MTU fill, Benford status; **no broadcast** |
| `cast` | `--target`, `--amount`, `--level`, `--rpc-url`, `--handoff` or `--keypair`, optional `--alt`, `--send` | Atomic fuzzy tx; refuses underfunded; signs; simulates; sends iff `--send` |
| `campaign` | same funding inputs + `--txs` / level + `--isolate-intent` (default true) | Multi-tx plan; decoys best-effort; real intent hard-fail |
| `info` | none | Version, license, threat model matrix, allowed decoy kinds |

Default RPC: `https://api.devnet.solana.com`. Cluster in handoff must match operator intent (warn on mismatch).

---

## 16. Resolved design ambiguities

These were decided while writing this spec so implementation does not stall on open questions:

1. **Jito:** stretch / optional `Submitter` feature — **not** required for v1 bar C completion.
2. **CPI wrapper usage:** keep on-chain; default SDK decoys use `noop_decoy`; `--via-router` opt-in for CPI path.
3. **Handoff secrets:** file paths only in v1 JSON (no embedded secret arrays).
4. **Cast send default:** require explicit `--send` to broadcast (simulate-by-default reduces accidental mainnet loss).
5. **Campaign isolation:** default `isolate-intent = true`.
6. **Root docs pointer:** **not** created — workspace root has no natural `docs/` tree; canonical spec lives only under `supersonic-tx/docs/superpowers/specs/`.
7. **Tip accounts:** configured via CLI/config allowlist plus cooked sinks; no hardcoded fake DEX program ids.
8. **Mainnet deploy:** optional; devnet + CI sufficient for “ship path” demo completeness.

---

## 17. Spec self-review record

Performed after draft:

- Scanned for `TBD` / `TODO` / placeholder language — none left as open work items; gaps are listed as concrete fixes in §12.
- Checked contradictions: single-cast atomicity vs campaign isolation — documented in §5; decoy realism vs old ARCHITECTURE Jupiter narrative — superseded by §7 / §11.
- Scope creep check: Jito, ephemeral programs, SPL graphs marked future/stretch in §13.
- Concreteness check: crates, CLI flags, handoff schema, milestones, success exits named.

**Self-review status:** complete. Spec is intended to be sufficient input for an implementation plan (writing-plans skill) without further product brainstorming.

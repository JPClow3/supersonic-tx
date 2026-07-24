# Superteam Earn — submission draft (noise bounty)

Copy/paste into the Earn form. Language mirrors the bounty brief, not internal SDD.

**Operator:** confirm Brazil regional eligibility before submit (checkbox / regional listing).

---

## Submission text

### What I built

**supersonic-tx** — a production-oriented **Rust** end-to-end toolkit that wraps a real Solana transfer in **realistic decoy noise** (not naive RNG). It targets algotraders, whales, and agents who want behavioral obscurity against automated graph/shape heuristics.

It is **not** a mixer, tumbler, shielded pool, or ZK privacy system. Threat model is honest: obscurity against automated heuristics — not cryptographic unlinkability.

### Repo / pin

| Field | Value |
| --- | --- |
| Repository | https://github.com/JPClow3/supersonic-tx |
| Branch (judge entry) | `feature/bar-c` |
| Release tag | [`v0.1.0-bar-c`](https://github.com/JPClow3/supersonic-tx/releases/tag/v0.1.0-bar-c) |
| Tag commit (peel) | identical to tag v0.1.0-bar-c tip (git rev-parse v0.1.0-bar-c^{}) |
| License | MIT |

### Which tool (bounty scope)

- **Primary:** `supersonic-tx` — fuzzy bundle builder + CLI (`cook` → `simulate` / `cast` / `campaign`) + shared Anchor router.
- **Integrated & composable:** `account-cooker` (same workspace) produces schema-v1 handoff JSON (`FeePayer` / `DecoySink` / `DrainTarget` + key paths under `--out-dir`). The cast/campaign path consumes that handoff; cooker is not a bolted-on demo script.

### Design choice

**Approach 1 — shared Anchor router + off-chain orchestrator.**

- On-chain: one global program ID (`GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`) with `noop_decoy` and opt-in `execute_fuzzy_bundle` CPI wrapper.
- Off-chain: Rust SDK/CLI chooses entropy, decoy counts, interleave order, ALT resolve, MTU shrink, sign/simulate/send.
- Shared program ID grows a common fingerprint (anonymity-set *shape*), while default intent remains a direct System Program transfer; `--via-router` is opt-in.

### Proof it is tested / deployable

| Evidence | Link / note |
| --- | --- |
| Green CI (workspace `rust` + dual-lock `sbf`) | https://github.com/JPClow3/supersonic-tx/actions/runs/30061536023 |
| Tests | `cargo test --workspace --locked` (+ standalone router crate tests). Reference Docker suite: **56** executable tests (50 workspace + 6 router). |
| Localnet smoke | **PASS** 2026-07-24 — Docker validator, deploy program, `cook` → `cast --via-router --send`. Details: [docs/smoke.md](smoke.md). |
| Reference cast signature (localnet) | `39iuxT1gGq2jsRzUwtb5441ME6aebnTuXhSzrd3yfFvttpGW5LBqGKnbsxbu7FvaTJh3ztek5EWjFw7cQqG7tpdE` — simulation OK, **484/1232** bytes, **6** decoys. |
| Devnet | **Not deployed** — needs funded deployer (public faucet may 429). Same program ID reserved; reproducible via Docker localnet per [docs/deploy.md](deploy.md). No invented explorer URLs. |

Judge path (60s): clone → checkout `v0.1.0-bar-c` → Docker `cargo test --workspace --locked` (see README). Full smoke: [docs/smoke.md](smoke.md).

### Realistic noise (not naive RNG)

- **Benford-ish** statistical SOL transfer amounts to cooker-funded sinks (or allowlisted `--tip`).
- **TrustedSystemAccount** sink provenance only — cooker `DecoySink` + non-empty `secret_key_path`, or tip allowlist; RPC validation rejects executable / non-system owners.
- **Fail-soft:** without validated sinks → CU/memo/router-only path (`without_transfer_noise`); no fake Jupiter / arbitrary custom program decoys on the safe builder path.
- **Campaign isolate-intent** (default `true`): statistical transfers stay out of the real-intent tx so a decoy failure does not abort the action.
- **MTU ≤ 1232:** shrink drops decoys in a defined order; **never** drops the real intent.

Levels: `light` | `standard` | `paranoid`.

### Threat model (honest)

| Helps against | Does not defeat |
| --- | --- |
| Naive wallet-graph clustering (partial) | Analyst who filters on known `PROGRAM_ID` |
| Simple CU / shape heuristics (partial) | Sponsor → cooker funding trace (always visible) |
| Single-obvious-instruction filters (partial) | Timing across campaign txs; mempool / copy-trade timing |
| | CEX / KYC / human review; unique target ix data |
| | Mixing, ZK, unlinkable funding |

Atomic `cast` decoys share fate with the real intent; use `campaign --isolate-intent true` when decoy failure must not abort the action.

### Brazil regional eligibility

**Operator confirms:** this submission is for the **Superteam Brazil** regional listing / BR-eligible Earn bounty. (Do not submit until residency/eligibility is verified.)

### One-liner for the form “summary” field

Rust E2E noise toolkit (`supersonic-tx` + integrated `account-cooker`): Approach 1 shared Anchor router + off-chain orchestrator; Benford sinks, TrustedSystemAccount fail-soft, isolate-intent campaigns, MTU≤1232; green CI + localnet cook→cast smoke; MIT; honest threat model (not a mixer).

---

## Optional Twitter / X thread draft

1/ Built **supersonic-tx** for the Superteam Brazil noise bounty — Rust end-to-end: cook accounts → cast a real transfer wrapped in realistic decoys (Benford amounts, fail-soft CU/memo, shared Anchor router). Not a mixer. Not ZK.

2/ Design: Approach 1 — one shared on-chain router + off-chain orchestrator. `account-cooker` is integrated (schema-v1 handoff), not a side script. Campaigns default to isolate-intent; payloads shrink to ≤1232 without dropping the real ix.

3/ Proof: green CI + localnet smoke (`cook` → `cast --via-router --send`). Devnet when a funded deployer is available — same program ID, no fake explorer links.

Repo (tag `v0.1.0-bar-c`): https://github.com/JPClow3/supersonic-tx

CI: https://github.com/JPClow3/supersonic-tx/actions/runs/30061536023

Localnet cast: `39iuxT1gGq2jsRzUwtb5441ME6aebnTuXhSzrd3yfFvttpGW5LBqGKnbsxbu7FvaTJh3ztek5EWjFw7cQqG7tpdE`

---

## Checklist before paste-submit

- [ ] Brazil eligibility confirmed by operator
- [x] Tag `v0.1.0-bar-c` retagged onto tip (Phase D) so judges opening the tag see Earn draft + Phase B smoke
- [ ] Devnet optional: only if funded deployer — then add Solscan/explorer links to README Deployments **and** this draft
- [x] Re-read bounty “great contribution” criteria once against this text (Rust E2E, tested/deployable, realistic noise, docs, MIT, cooker composability)
- [x] No secrets / keypairs in repo or form attachments (operator keypaths gitignored under `.tmp-operator-sim/`)
- [x] README Verification CI URL matches the run cited here (`30061536023`)

---

## What you do next (Earn submit — operator only)

1. Open Superteam Earn → Brazil noise bounty → **Submit**.
2. Paste from this file: summary one-liner + “Submission text” sections (repo URL, tag `v0.1.0-bar-c`, CI + localnet cast proof).
3. Confirm **Brazil / regional eligibility** checkbox.
4. Click **Submit** yourself — do not ask an agent to submit on your behalf.

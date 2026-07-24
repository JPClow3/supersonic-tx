# Supersonic-tx Bar C Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship production-ready fuzzy transaction bundles: real Anchor router ID, signed V0 send, fail-soft decoys, `account-cooker` handoff, multi-tx campaign isolation, real ALT fetch, workspace CI, and honest MIT/docs — matching design bar C.

**Architecture:** Approach 1 — one global shared Anchor router (`programs/supersonic-tx`) plus an off-chain orchestrator (`supersonic-tx-sdk` + CLI). `account-cooker` produces handoff JSON (keypair *paths* only); the SDK builds fail-soft decoys, campaigns, ALT-aware V0 messages, signs, simulates, and sends only when `--send` is set. Jito is stretch-only and not required for v1 done.

**Tech Stack:** Rust 2021 workspace, Anchor `0.30.1`, `solana-sdk`/`solana-client`/`solana-program` `~1.18`, `clap` 4, `serde`/`serde_json`, `bincode` 1.3, `tokio`, `rand`/`rand_distr`, `solana-program-test`, GitHub Actions.

## Global Constraints

- Workspace root for all product work: `H:\Code\Pessoais\SP - Solana\supersonic-tx\` (sibling empty stub `..\account-cooker\` is **not** the crate path — implement at `crates/account-cooker/`).
- Pins: Anchor `0.30.1`, Solana crates `~1.18`, MIT license, binary name `supersonic-tx`.
- V0 compile must use Solana 1.18 API: `solana_sdk::message::v0::Message::try_compile` → `VersionedMessage::V0(...)` (do **not** call nonexistent `VersionedMessage::try_compile`).
- `MAX_TX_PAYLOAD_BYTES = 1232`; serialize size checks with `bincode`.
- Fail-soft decoys only; never fake Jupiter/DEX destinations; never transfer to executable program IDs; never broadcast `Signature::default()`.
- Handoff JSON `schema_version = 1`: `secret_key_path` relative paths only — no embedded secrets in v1.
- CLI: `--send` opt-in for broadcast; `campaign --isolate-intent` default **true**; default RPC `https://api.devnet.solana.com`.
- Jito / ephemeral programs: out of scope for v1 done.
- SPL decoy graphs: v1 non-goal for full ops; post bar-C SDK routes landed (see README Roadmap).
- Spec authority: `docs/superpowers/specs/2026-07-23-supersonic-tx-design.md`.
- Exit bar C: `cargo test --workspace` + `anchor build` green; cooker + campaign + ALT + CI + documented deploy path.

---

## File Structure Map

| Path | Action | Responsibility |
| --- | --- | --- |
| `Cargo.toml` | Modify | Add `crates/account-cooker` member; workspace `bincode = "1.3"`; optional `account-cooker` path dep |
| `Cargo.lock` | Create/commit | Lockfile after deps resolve |
| `.gitignore` | Create | `target/`, `.anchor/`, keypair dirs, `.agents/` quarantine |
| `.github/workflows/ci.yml` | Create | `cargo test --workspace` + `anchor build` |
| `LICENSE-MIT` | Modify | Fix `INCLUDINGRemind` → `INCLUDING` |
| `README.md` | Rewrite | Honest threat model, CLI contract, deploy, no agent victory claims |
| `ARCHITECTURE.md` | Rewrite | Align with Approach 1 + fail-soft sinks (remove Jupiter decoy narrative) |
| `Anchor.toml` | Modify | Real `supersonic_tx` program IDs for localnet/devnet |
| `programs/supersonic-tx/src/lib.rs` | Modify | Real `declare_id!`; keep `noop_decoy` / `execute_fuzzy_bundle` / events / errors |
| `programs/supersonic-tx/tests/router_tests.rs` | Replace | Real `solana-program-test` coverage (replace struct-only asserts) |
| `programs/supersonic-tx/Cargo.toml` | Modify | Add `solana-program-test` / test deps as needed |
| `crates/supersonic-tx-core/src/lib.rs` | Modify | Real `PROGRAM_ID_STR` + `PROGRAM_ID: Pubkey` helper |
| `crates/supersonic-tx-core/src/types.rs` | Modify | Extend `SupersonicError` (unsigned, underfunded, alt, campaign) |
| `crates/supersonic-tx-sdk/Cargo.toml` | Modify | `bincode.workspace`, keep `solana-client` |
| `crates/supersonic-tx-sdk/src/lib.rs` | Modify | Export new modules + `MemoNoise` |
| `crates/supersonic-tx-sdk/src/noise.rs` | Modify | Tip/sink allowlist; level counts per spec; forbid empty sinks |
| `crates/supersonic-tx-sdk/src/builder.rs` | Modify | V0 compile, priority shrink, size-check without claiming signed |
| `crates/supersonic-tx-sdk/src/alt.rs` | Create | `AltResolver` RPC fetch → `AddressLookupTableAccount` |
| `crates/supersonic-tx-sdk/src/sign.rs` | Create | `sign_versioned_tx`, `simulate_and_send` |
| `crates/supersonic-tx-sdk/src/campaign.rs` | Create | `CampaignPlan` / `CampaignPlanner` / `PlannedTx` |
| `crates/account-cooker/` | Create | Full crate: types, cooker, fund/drain, handoff I/O |
| `crates/supersonic-tx-cli/src/main.rs` | Modify | Subcommands `cook`, `simulate`, `cast`, `campaign`, `info` |
| `crates/supersonic-tx-cli/Cargo.toml` | Modify | `bincode.workspace`, `account-cooker`, `serde_json` |
| `docs/deploy.md` | Create | Devnet deploy checklist (program id sync + smoke) |
| `docs/internal/agents-archive/README.md` | Create | Quarantine note if moving `.agents/` (or rely on `.gitignore`) |
| `../account-cooker/README.md` | Create (optional) | Pointer: real crate lives in `supersonic-tx/crates/account-cooker` |

---

# Milestone M0 — Spec lock & hygiene

### Task 1: License typo, gitignore, agents quarantine

**Files:**
- Modify: `LICENSE-MIT`
- Create: `.gitignore`
- Create: `docs/internal/agents-archive/README.md` (if relocating) **or** only ignore `.agents/`
- Optional: move `.agents/` → `docs/internal/agents-archive/.agents/`

**Interfaces:**
- Consumes: none
- Produces: corrected MIT text; `.agents/` excluded from product narrative / git tracking

- [ ] **Step 1: Write a failing hygiene check script assertion (document as test)**

Create `scripts/check_hygiene.sh` is optional; prefer a tiny Rust-free verification by grepping. For TDD without a harness, add a CLI unit later — for this task use shell checks as the gate.

- [ ] **Step 2: Run check to verify current failure**

Run from `supersonic-tx/`:

```bash
rg -n "INCLUDINGRemind" LICENSE-MIT
```

Expected: match on line with `INCLUDINGRemind`.

- [ ] **Step 3: Fix LICENSE and add `.gitignore`**

In `LICENSE-MIT`, replace:

```text
IMPLIED, INCLUDINGRemind BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
```

with:

```text
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
```

Create `.gitignore`:

```gitignore
/target/
**/*.rs.bk
.anchor/
.DS_Store
*.swp
# Deploy & operator secrets
target/deploy/*-keypair.json
**/keys/
**/handoff-*.json
!docs/**
# Agent process notes are non-authoritative (spec §11.1)
.agents/
```

Create `docs/internal/agents-archive/README.md`:

```markdown
# Agents archive (non-authoritative)

Files under `.agents/` (ignored) or archived here are **process notes from agent runs**.
They are **not** proof of bounty completion. Source of truth: `cargo test --workspace`,
`anchor build`, and this repository's product docs / CI.
```

Optionally move the existing `.agents/` tree into `docs/internal/agents-archive/` then keep `.agents/` in gitignore for future local notes. Product README must not link to them as victory proof.

- [ ] **Step 4: Re-run checks**

```bash
rg -n "INCLUDINGRemind" LICENSE-MIT; echo "exit:$?"
# expect no matches
test -f .gitignore && echo "gitignore ok"
```

Expected: no `INCLUDINGRemind`; gitignore present.

- [ ] **Step 5: Commit**

```bash
git add LICENSE-MIT .gitignore docs/internal/agents-archive/README.md
git commit -m "chore: fix MIT typo and quarantine agent notes"
```

---

### Task 2: Workspace `bincode` pin and SDK dependency wire-up

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/supersonic-tx-sdk/Cargo.toml`
- Modify: `crates/supersonic-tx-cli/Cargo.toml`

**Interfaces:**
- Consumes: existing workspace members
- Produces: `bincode` available as `bincode.workspace = true` in SDK + CLI

- [ ] **Step 1: Write failing compile probe**

In `crates/supersonic-tx-sdk/src/builder.rs` the code already calls `bincode::serialize` but SDK `Cargo.toml` lacks `bincode`. Confirm:

```bash
cargo check -p supersonic-tx-sdk 2>&1 | head -n 40
```

Expected: FAIL with unresolved `bincode` (or already broken via `VersionedMessage::try_compile`).

- [ ] **Step 2: Record failure output**

Keep the error text; both `bincode` and `try_compile` may appear. This task only fixes deps; Task 4 fixes the API.

- [ ] **Step 3: Minimal dep fix**

In root `Cargo.toml` under `[workspace.dependencies]` add:

```toml
bincode = "1.3"
```

In `crates/supersonic-tx-sdk/Cargo.toml` dependencies add:

```toml
bincode.workspace = true
```

In `crates/supersonic-tx-cli/Cargo.toml` replace `bincode = "1.3"` with:

```toml
bincode.workspace = true
serde_json.workspace = true
```

Ensure root already has `serde_json` (it does).

- [ ] **Step 4: Re-check SDK deps resolve**

```bash
cargo metadata --no-deps -q >/dev/null && cargo check -p supersonic-tx-sdk 2>&1 | rg "bincode|try_compile|error" | head -n 20
```

Expected: no “can't find crate `bincode`”; `try_compile` / other errors may remain until Task 4.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/supersonic-tx-sdk/Cargo.toml crates/supersonic-tx-cli/Cargo.toml
git commit -m "build: add workspace bincode for SDK and CLI"
```

---

# Milestone M1 — Core truthfulness

### Task 3: Real program ID plumbing (local keypair + synced constants)

**Files:**
- Create: `target/deploy/supersonic_tx-keypair.json` (generate; do **not** commit keypair — keep gitignored)
- Modify: `programs/supersonic-tx/src/lib.rs` (`declare_id!`)
- Modify: `crates/supersonic-tx-core/src/lib.rs` (`PROGRAM_ID_STR`, `PROGRAM_ID`)
- Modify: `Anchor.toml` `[programs.localnet]` / `[programs.devnet]`
- Test: `crates/supersonic-tx-core/src/lib.rs` (inline tests) or new `tests/program_id.rs`

**Interfaces:**
- Consumes: Solana keypair pubkey base58
- Produces:
  - `pub const PROGRAM_ID_STR: &str = "<real base58>";`
  - `pub fn program_id() -> Pubkey` (or `lazy_static`/const parse)
  - Matching `declare_id!("...")` and Anchor.toml entries
  - **Never** `Super11111111111111111111111111111111111111`

- [ ] **Step 1: Write the failing test**

Add to `crates/supersonic-tx-core/src/lib.rs`:

```rust
#[cfg(test)]
mod program_id_tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    #[test]
    fn program_id_is_not_placeholder() {
        assert_ne!(
            PROGRAM_ID_STR,
            "Super11111111111111111111111111111111111111"
        );
        let pk = Pubkey::from_str(PROGRAM_ID_STR).expect("PROGRAM_ID_STR must be valid base58");
        assert_eq!(pk, program_id());
    }
}
```

Also add:

```rust
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID_STR).expect("PROGRAM_ID_STR must be valid")
}
```

(Keep stub `PROGRAM_ID_STR` temporarily so the test compiles and fails the assert.)

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p supersonic-tx-core program_id_is_not_placeholder -- --nocapture
```

Expected: FAIL assertion `PROGRAM_ID_STR != Super111...`.

- [ ] **Step 3: Generate key and sync IDs**

```bash
mkdir -p target/deploy
solana-keygen new --no-bip39-passphrase -o target/deploy/supersonic_tx-keypair.json --force
solana-keygen pubkey target/deploy/supersonic_tx-keypair.json
```

Copy the printed pubkey into:

1. `declare_id!("<PUBKEY>");` in `programs/supersonic-tx/src/lib.rs`
2. `PROGRAM_ID_STR` in `crates/supersonic-tx-core/src/lib.rs`
3. `Anchor.toml`:

```toml
[programs.localnet]
supersonic_tx = "<PUBKEY>"

[programs.devnet]
supersonic_tx = "<PUBKEY>"
```

Update `AnchorRouterNoise::default()` consumers to keep using `PROGRAM_ID_STR` (already does).

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p supersonic-tx-core program_id_is_not_placeholder -- --nocapture
rg -n "Super111" programs/supersonic-tx/src/lib.rs crates/supersonic-tx-core/src/lib.rs Anchor.toml
```

Expected: test PASS; no `Super111` in those three files.

- [ ] **Step 5: Commit**

```bash
git add programs/supersonic-tx/src/lib.rs crates/supersonic-tx-core/src/lib.rs Anchor.toml
git commit -m "feat: replace placeholder program id with generated key pubkey"
```

---

### Task 4: Fix V0 message compile + unsigned size-check path

**Files:**
- Modify: `crates/supersonic-tx-sdk/src/builder.rs`
- Modify: `crates/supersonic-tx-core/src/types.rs` (add errors if needed)
- Test: `crates/supersonic-tx-sdk/src/builder.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `payer: Pubkey`, instructions, `&[AddressLookupTableAccount]`, `Hash`
- Produces:
  - `FuzzyBundleBuilder::compile_v0_message(...) -> Result<VersionedMessage, SupersonicError>`
  - `FuzzyBundleBuilder::estimate_tx_size(message: &VersionedMessage) -> Result<usize, SupersonicError>` using placeholder signature slots **only for size estimation**
  - `FuzzyBundleBuilder::build_versioned_message(...)` shrink loop returning `VersionedMessage` (not a falsely “ready to send” signed tx)
  - Signing moved to Task 12 (`sign_versioned_tx`)

- [ ] **Step 1: Write the failing test**

Replace/extend builder tests:

```rust
#[test]
fn compiles_v0_message_without_versioned_message_try_compile() {
    use solana_sdk::message::VersionedMessage;
    let payer = Pubkey::new_unique();
    let target = Pubkey::new_unique();
    let ix = solana_sdk::system_instruction::transfer(&payer, &target, 10_000);
    let builder = FuzzyBundleBuilder::new(payer, ObfuscationLevel::Light)
        .add_target_instruction(ix);
    let msg = builder
        .build_versioned_message(Hash::new_unique(), &[])
        .expect("v0 compile");
    match msg {
        VersionedMessage::V0(_) => {}
        VersionedMessage::Legacy(_) => panic!("expected V0"),
    }
    let size = FuzzyBundleBuilder::estimate_tx_size(&msg).unwrap();
    assert!(size <= MAX_TX_PAYLOAD_BYTES);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p supersonic-tx-sdk compiles_v0_message_without_versioned_message_try_compile -- --nocapture
```

Expected: FAIL (missing method and/or `VersionedMessage::try_compile`).

- [ ] **Step 3: Write minimal implementation**

Update imports in `builder.rs`:

```rust
use solana_sdk::message::{v0, VersionedMessage};
use solana_sdk::signature::Signature;
// remove reliance on VersionedMessage::try_compile
```

Implement:

```rust
pub fn compile_v0_message(
    payer: &Pubkey,
    instructions: &[Instruction],
    address_lookup_table_accounts: &[AddressLookupTableAccount],
    recent_blockhash: Hash,
) -> Result<VersionedMessage, SupersonicError> {
    let message = v0::Message::try_compile(
        payer,
        instructions,
        address_lookup_table_accounts,
        recent_blockhash,
    )
    .map_err(|e| {
        SupersonicError::InvalidDecoyConfig(format!("v0::Message::try_compile failed: {e}"))
    })?;
    Ok(VersionedMessage::V0(message))
}

pub fn estimate_tx_size(message: &VersionedMessage) -> Result<usize, SupersonicError> {
    let num_signatures = message.header().num_required_signatures as usize;
    let tx = VersionedTransaction {
        signatures: vec![Signature::default(); num_signatures],
        message: message.clone(),
    };
    let serialized_bytes = bincode::serialize(&tx)
        .map_err(|e| SupersonicError::SerializationError(e.to_string()))?;
    Ok(serialized_bytes.len())
}

pub fn build_versioned_message(
    &self,
    recent_blockhash: Hash,
    address_lookup_table_accounts: &[AddressLookupTableAccount],
) -> Result<VersionedMessage, SupersonicError> {
    let mut manifest = self.build_manifest()?;
    loop {
        let instructions = Self::assemble_instructions(&manifest);
        let message = Self::compile_v0_message(
            &self.payer,
            &instructions,
            address_lookup_table_accounts,
            recent_blockhash,
        )?;
        let size = Self::estimate_tx_size(&message)?;
        if size <= MAX_TX_PAYLOAD_BYTES {
            return Ok(message);
        }
        if !Self::shrink_decoys(&mut manifest) {
            return Err(SupersonicError::TransactionSizeExceeded(size));
        }
    }
}
```

Keep a thin deprecated wrapper if CLI still calls `build_versioned_transaction` — change it to return unsigned estimation-only OR remove and update call sites in same milestone (prefer remove/replace with `build_versioned_message` + later `sign_versioned_tx`).

Implement `shrink_decoys` priority (spec §7.4): drop statistical transfers first, then memo, then extra router noops; never drop all CU padding if a CU ix exists; never drop target ixs. Heuristic: classify by `program_id` (system transfer = statistical, memo program, router `PROGRAM_ID`, compute budget).

```rust
fn shrink_decoys(manifest: &mut BundleManifest) -> bool {
    // 1) pop a system-program transfer decoy if any
    // 2) else pop a memo decoy
    // 3) else pop a router noop if >1 remain
    // 4) else pop any remaining decoy except last compute-budget ix
    // return false if nothing removable
}
```

Re-shuffle `execution_order` after shrink.

- [ ] **Step 4: Run tests**

```bash
cargo test -p supersonic-tx-sdk -- --nocapture
```

Expected: PASS including MTU overflow test (update it to call `build_versioned_message`).

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-sdk/src/builder.rs crates/supersonic-tx-core/src/types.rs
git commit -m "fix: compile V0 via v0::Message::try_compile and separate size estimate"
```

---

### Task 5: Realistic fail-soft decoy destinations (tips + cooked sinks)

**Files:**
- Modify: `crates/supersonic-tx-sdk/src/noise.rs`
- Modify: `crates/supersonic-tx-sdk/src/builder.rs` (inject sinks into `StatisticalTransferNoise`)
- Modify: `crates/supersonic-tx-sdk/src/lib.rs` (export `MemoNoise`)
- Test: `noise.rs` unit tests

**Interfaces:**
- Consumes: allowlisted tip pubkeys + cooked `DecoySink` pubkeys from handoff
- Produces:
  - `StatisticalTransferNoise::from_sinks(sinks: Vec<Pubkey>)`
  - `StatisticalTransferNoise::default_tip_allowlist()` — **real tip/fee receivers or empty requiring injection**; **remove** bogus Jupiter key `JUP6LkbZbjS1jKKwapdH67yN5k8u4nKq1X4fD6F9yM5`
  - Level counts for `AnchorRouterNoise` per spec: Light `0..=1`, Standard `1`, Paranoid `1..=2` (use fixed Standard=1; Light random 0–1 or 1; Paranoid 2)
  - Refuse generation when destinations empty: return empty vec **or** builder error `InvalidDecoyConfig("no fail-soft sinks")` when level requires transfers

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn statistical_noise_rejects_fake_jupiter_default() {
    let noise = StatisticalTransferNoise::default_tip_allowlist();
    for d in &noise.decoy_destinations {
        let s = d.to_string();
        assert!(!s.starts_with("JUP6"), "forbidden fake Jupiter destination: {s}");
        // destinations must be non-executable wallets; we cannot check executable off-chain
        // without RPC — enforce allowlist membership instead in builder tests.
    }
}

#[test]
fn statistical_noise_uses_injected_sinks() {
    let sink = Pubkey::new_unique();
    let noise = StatisticalTransferNoise::from_sinks(vec![sink]);
    let payer = Pubkey::new_unique();
    let decoys = noise.generate_decoys(&payer, ObfuscationLevel::Light);
    assert_eq!(decoys.len(), 1);
    assert_eq!(decoys[0].accounts[1].pubkey, sink);
}

#[test]
fn router_noop_counts_match_spec_standard() {
    let payer = Pubkey::new_unique();
    let noise = AnchorRouterNoise::default();
    assert_eq!(noise.generate_decoys(&payer, ObfuscationLevel::Standard).len(), 1);
}
```

- [ ] **Step 2: Run tests to verify failure**

```bash
cargo test -p supersonic-tx-sdk statistical_noise_rejects_fake_jupiter_default router_noop_counts_match_spec_standard -- --nocapture
```

Expected: FAIL (old Jupiter defaults / Standard count == 2).

- [ ] **Step 3: Implement allowlist + count fixes**

Replace `default_mainnet_destinations` with:

```rust
impl StatisticalTransferNoise {
    pub fn from_sinks(decoy_destinations: Vec<Pubkey>) -> Self {
        Self { decoy_destinations }
    }

    /// Public tip / fee sink allowlist for fail-soft SOL transfers.
    /// Operators should prefer cooked DecoySinks from account-cooker.
    /// v1 ships an empty default; CLI/SDK inject sinks from handoff or `--tip`.
    pub fn default_tip_allowlist() -> Self {
        Self::from_sinks(Vec::new())
    }

    pub fn with_tips(mut self, tips: impl IntoIterator<Item = Pubkey>) -> Self {
        self.decoy_destinations.extend(tips);
        self
    }
}
```

Update `FuzzyBundleBuilder::new` to **not** call Jupiter defaults. Prefer:

```rust
generators: vec![
    Box::new(ComputeBudgetNoise::default()),
    Box::new(AnchorRouterNoise::default()),
    Box::new(MemoNoise::default()),
],
```

and add `.with_sinks(sinks: Vec<Pubkey>)` that inserts `StatisticalTransferNoise::from_sinks(sinks)` when non-empty.

Fix `AnchorRouterNoise` counts:

```rust
let count = match level {
    ObfuscationLevel::Light => 1,      // within 0–1; use 1 for demo density
    ObfuscationLevel::Standard => 1,
    ObfuscationLevel::Paranoid => 2,
};
```

Export `MemoNoise` from `lib.rs`.

- [ ] **Step 4: Run tests**

```bash
cargo test -p supersonic-tx-sdk -- --nocapture
```

Expected: PASS; no Jupiter pubkey in source:

```bash
rg -n "JUP6Lkb|fake Jupiter|Raydium AMM" crates/supersonic-tx-sdk/src/noise.rs
```

Expected: no matches for fake destinations.

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-sdk/src/noise.rs crates/supersonic-tx-sdk/src/builder.rs crates/supersonic-tx-sdk/src/lib.rs
git commit -m "fix: use fail-soft tip/sink decoys instead of fake DEX destinations"
```

---

### Task 6: MTU shrink priority unit tests

**Files:**
- Modify: `crates/supersonic-tx-sdk/src/builder.rs` tests
- Test: same

**Interfaces:**
- Consumes: `shrink_decoys` from Task 4
- Produces: documented priority — statistical → memo → extra noop; retain ≥1 CU when present

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn shrink_drops_statistical_before_memo() {
    let mut manifest = BundleManifest::new(ObfuscationLevel::Standard);
    let payer = Pubkey::new_unique();
    let sink = Pubkey::new_unique();
    manifest.decoy_instructions = vec![
        solana_sdk::system_instruction::transfer(&payer, &sink, 1000),
        Instruction {
            program_id: Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr").unwrap(),
            accounts: vec![],
            data: b"x".to_vec(),
        },
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(200_000),
    ];
    assert!(FuzzyBundleBuilder::shrink_decoys_for_test(&mut manifest));
    assert!(
        manifest
            .decoy_instructions
            .iter()
            .all(|ix| ix.program_id != solana_sdk::system_program::id()
                || ix.data.first() != Some(&2)), // transfer ix layout: still prefer checking accounts len
    );
    // After one shrink, memo + CU remain
    assert_eq!(manifest.decoy_instructions.len(), 2);
}
```

Expose `shrink_decoys` as `pub(crate) fn shrink_decoys` and `#[cfg(test)] pub fn shrink_decoys_for_test`.

- [ ] **Step 2: Run test (expect fail if priority wrong)**

```bash
cargo test -p supersonic-tx-sdk shrink_drops_statistical_before_memo -- --nocapture
```

- [ ] **Step 3: Adjust shrink implementation until correct**

Implement classification helpers in `builder.rs` and drop in order.

- [ ] **Step 4: Run tests**

```bash
cargo test -p supersonic-tx-sdk shrink_drops_statistical_before_memo -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-sdk/src/builder.rs
git commit -m "test: enforce MTU shrink priority for fail-soft decoys"
```

---

# Milestone M2 — account-cooker

### Task 7: Handoff schema types + serde round-trip

**Files:**
- Create: `crates/account-cooker/Cargo.toml`
- Create: `crates/account-cooker/src/lib.rs`
- Create: `crates/account-cooker/src/types.rs`
- Modify: root `Cargo.toml` members += `"crates/account-cooker"`

**Interfaces:**
- Consumes: `serde`, `serde_json`
- Produces:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookedRole { FeePayer, DecoySink, DrainTarget }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CookedAccount {
    pub role: CookedRole,
    pub pubkey: String,
    pub secret_key_path: Option<String>,
    pub funded_lamports: u64,
    pub min_required_lamports: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffBundle {
    pub schema_version: u32, // must be 1
    pub cluster: String,
    pub created_at_unix: i64,
    pub sponsor_pubkey: String,
    pub accounts: Vec<CookedAccount>,
    pub warnings: Vec<String>,
}
```

- [ ] **Step 1: Write the failing test**

In `crates/account-cooker/src/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_json_round_trip_schema_v1() {
        let h = HandoffBundle {
            schema_version: 1,
            cluster: "devnet".into(),
            created_at_unix: 1721750400,
            sponsor_pubkey: "11111111111111111111111111111111".into(),
            accounts: vec![CookedAccount {
                role: CookedRole::FeePayer,
                pubkey: "FeePayer111111111111111111111111111111111".into(),
                secret_key_path: Some("keys/fee_payer.json".into()),
                funded_lamports: 50_000_000,
                min_required_lamports: 10_000_000,
            }],
            warnings: vec![],
        };
        let json = serde_json::to_string_pretty(&h).unwrap();
        assert!(json.contains("\"schema_version\": 1"));
        assert!(!json.contains("secret\":"));
        let back: HandoffBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }
}
```

Scaffold crate so test compiles against missing types (will fail compile first — that's the red).

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p account-cooker handoff_json_round_trip_schema_v1 -- --nocapture
```

Expected: FAIL (package missing / type missing).

- [ ] **Step 3: Scaffold crate + implement types**

`crates/account-cooker/Cargo.toml`:

```toml
[package]
name = "account-cooker"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Fresh Solana keypairs, sponsor funding, and handoff JSON for supersonic-tx"

[dependencies]
solana-sdk.workspace = true
solana-client.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
tracing.workspace = true
```

Root `Cargo.toml` members insert `"crates/account-cooker"`.

Implement types exactly as Interfaces; `lib.rs`: `pub mod types; pub use types::*;`

- [ ] **Step 4: Run test**

```bash
cargo test -p account-cooker handoff_json_round_trip_schema_v1 -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/account-cooker
git commit -m "feat: add account-cooker handoff schema v1"
```

---

### Task 8: Cooker keygen, write key dir, handoff file I/O

**Files:**
- Create: `crates/account-cooker/src/cooker.rs`
- Modify: `crates/account-cooker/src/lib.rs`
- Test: `cooker.rs` unit tests (no RPC)

**Interfaces:**
- Consumes: `HandoffBundle` types
- Produces:

```rust
pub struct CookerConfig {
    pub cluster: String,
    pub n_sinks: usize,
    pub fund_fee_payer_lamports: u64,
    pub fund_sink_lamports: u64,
    pub min_fee_payer_lamports: u64,
    pub min_sink_lamports: u64,
}

pub struct Cooker {
    // sponsor Keypair held for fund/drain later
}

impl Cooker {
    pub fn new_offline(sponsor_pubkey: Pubkey) -> Self;
    pub fn generate(&self, cfg: &CookerConfig) -> Result<(HandoffBundle, Vec<(CookedRole, Keypair)>), CookerError>;
    pub fn write_keypair_dir(out_dir: &Path, pairs: &[(CookedRole, Keypair)]) -> Result<Vec<CookedAccount>, CookerError>;
    pub fn write_handoff(path: &Path, handoff: &HandoffBundle) -> Result<(), CookerError>;
    pub fn load_handoff(path: &Path) -> Result<HandoffBundle, CookerError>;
    pub fn resolve_keypairs(handoff: &HandoffBundle, handoff_dir: &Path) -> Result<Vec<Keypair>, CookerError>;
}
```

Secret rule: write Solana JSON keypair files under `{out_dir}/keys/`; handoff stores relative `keys/...` only.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn generate_unique_keys_and_write_handoff_paths_only() {
    let dir = tempfile::tempdir().unwrap();
    let sponsor = Keypair::new();
    let cooker = Cooker::new_offline(sponsor.pubkey());
    let cfg = CookerConfig {
        cluster: "devnet".into(),
        n_sinks: 2,
        fund_fee_payer_lamports: 50_000_000,
        fund_sink_lamports: 2_000_000,
        min_fee_payer_lamports: 10_000_000,
        min_sink_lamports: 890_880,
    };
    let (mut handoff, pairs) = cooker.generate(&cfg).unwrap();
    let accounts = Cooker::write_keypair_dir(dir.path(), &pairs).unwrap();
    handoff.accounts = accounts;
    let path = dir.path().join("handoff-1.json");
    Cooker::write_handoff(&path, &handoff).unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(!raw.contains(&format!("{:?}", pairs[0].1.to_bytes())));
    assert!(raw.contains("keys/"));
    let loaded = Cooker::load_handoff(&path).unwrap();
    assert_eq!(loaded.schema_version, 1);
    let kps = Cooker::resolve_keypairs(&loaded, dir.path()).unwrap();
    assert_eq!(kps.len(), loaded.accounts.len());
}
```

Add `tempfile = "3"` as dev-dependency on account-cooker.

- [ ] **Step 2: Run test (fail)**

```bash
cargo test -p account-cooker generate_unique_keys_and_write_handoff_paths_only -- --nocapture
```

- [ ] **Step 3: Implement generate/write/load/resolve**

Use `solana_sdk::signature::{Keypair, Signer}` and `write_keypair_file` / `read_keypair_file`. File naming: `keys/fee_payer.json`, `keys/sink_{i}.json`, optional `keys/drain_target.json`.

- [ ] **Step 4: Run test (pass)**

```bash
cargo test -p account-cooker generate_unique_keys_and_write_handoff_paths_only -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/account-cooker
git commit -m "feat: account-cooker keygen and handoff file I/O"
```

---

### Task 9: Fund, drain, reuse warnings, underfund refuse

**Files:**
- Modify: `crates/account-cooker/src/cooker.rs`
- Create: `crates/account-cooker/src/error.rs`
- Test: unit tests with mockable trait **or** pure functions for shortfall math + reuse detection; RPC fund behind `#[cfg]` integration later

**Interfaces:**
- Consumes: `RpcClient` (blocking or async nonblocking)
- Produces:

```rust
impl Cooker {
    pub async fn fund_accounts(
        &self,
        rpc: &RpcClient,
        sponsor: &Keypair,
        handoff: &HandoffBundle,
        handoff_dir: &Path,
    ) -> Result<(), CookerError>;

    pub async fn drain(
        &self,
        rpc: &RpcClient,
        handoff: &HandoffBundle,
        handoff_dir: &Path,
        destination: &Pubkey,
    ) -> Result<(), CookerError>;

    pub fn assert_funded_for_cast(
        handoff: &HandoffBundle,
        estimated_fees: u64,
    ) -> Result<(), CookerError>;

    pub fn detect_reuse_warnings(out_dir: &Path, pubkeys: &[Pubkey]) -> Vec<String>;
}
```

`assert_funded_for_cast`: for each account, require `funded_lamports >= min_required_lamports`; fee payer also `>= min_required + estimated_fees`. Error lists shortfalls.

Reuse: if `keys/*.json` already exists for same pubkey path or duplicate pubkeys in one handoff → push into `warnings` and print path for CLI.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn assert_funded_refuses_shortfall() {
    let handoff = HandoffBundle {
        schema_version: 1,
        cluster: "devnet".into(),
        created_at_unix: 1,
        sponsor_pubkey: "x".into(),
        accounts: vec![CookedAccount {
            role: CookedRole::FeePayer,
            pubkey: "p".into(),
            secret_key_path: Some("keys/fee_payer.json".into()),
            funded_lamports: 1_000,
            min_required_lamports: 10_000_000,
        }],
        warnings: vec![],
    };
    let err = Cooker::assert_funded_for_cast(&handoff, 5_000).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("shortfall") || msg.contains("underfunded"));
}

#[test]
fn detect_reuse_warns_on_duplicate_pubkey() {
    let pk = Pubkey::new_unique();
    let warnings = Cooker::detect_reuse_warnings(Path::new("/tmp"), &[pk, pk]);
    assert!(!warnings.is_empty());
}
```

- [ ] **Step 2: Run fail**

```bash
cargo test -p account-cooker assert_funded_refuses_shortfall detect_reuse_warns_on_duplicate_pubkey -- --nocapture
```

- [ ] **Step 3: Implement**

Implement error enum:

```rust
#[derive(thiserror::Error, Debug)]
pub enum CookerError {
    #[error("underfunded handoff: {0}")]
    Underfunded(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("rpc: {0}")]
    Rpc(String),
    #[error("serde: {0}")]
    Serde(String),
    #[error("keypair: {0}")]
    Keypair(String),
}
```

`fund_accounts`: for each cooked account, `system_instruction::transfer` from sponsor for `funded_lamports`, sign+send (skip in unit tests — gate RPC methods behind integration or test with `solana-test-validator` documented in README). Unit coverage focuses on assert/reuse.

- [ ] **Step 4: Run pass**

```bash
cargo test -p account-cooker -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/account-cooker
git commit -m "feat: cooker underfund checks, reuse warnings, fund/drain APIs"
```

---

### Task 10: CLI `cook` subcommand

**Files:**
- Modify: `crates/supersonic-tx-cli/src/main.rs`
- Modify: `crates/supersonic-tx-cli/Cargo.toml` (`account-cooker` path dep)

**Interfaces:**
- Consumes: `Cooker`, `CookerConfig`
- Produces: CLI

```text
supersonic-tx cook \
  --sponsor-keypair <PATH> \
  --out-dir <DIR> \
  --rpc-url <URL> \
  --sinks <N> \
  --fee-payer-lamports <u64> \
  --sink-lamports <u64> \
  --cluster <name>
```

Writes `{out_dir}/keys/*` and `{out_dir}/handoff-<unix_ts>.json`. Funding requires RPC; add `--dry-run` that skips fund but still writes keypairs + handoff with configured lamport fields (document that dry-run is not cast-ready until funded).

- [ ] **Step 1: Write failing clap test**

```rust
#[test]
fn test_cli_parse_cook() {
    let args = vec![
        "supersonic-tx", "cook",
        "--sponsor-keypair", "/tmp/sponsor.json",
        "--out-dir", "/tmp/cook",
        "--rpc-url", "https://api.devnet.solana.com",
        "--sinks", "2",
    ];
    let cli = Cli::try_parse_from(args).expect("cook parse");
    match cli.command {
        Commands::Cook { sponsor_keypair, out_dir, sinks, .. } => {
            assert_eq!(sponsor_keypair, "/tmp/sponsor.json");
            assert_eq!(out_dir, "/tmp/cook");
            assert_eq!(sinks, 2);
        }
        _ => panic!("expected Cook"),
    }
}
```

- [ ] **Step 2: Run fail**

```bash
cargo test -p supersonic-tx-cli test_cli_parse_cook -- --nocapture
```

- [ ] **Step 3: Implement Cook variant + handler**

Wire `account-cooker` dependency:

```toml
account-cooker = { path = "../account-cooker" }
```

Implement command handler: generate → write keys → optional fund → write handoff → print path + warnings to stderr.

- [ ] **Step 4: Run pass**

```bash
cargo test -p supersonic-tx-cli test_cli_parse_cook -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-cli crates/account-cooker Cargo.toml
git commit -m "feat: CLI cook subcommand writing handoff JSON"
```

---

# Milestone M3 — Program tests + deployability

### Task 11: `solana-program-test` for noop, decoy_count==0, CPI path

**Files:**
- Replace: `programs/supersonic-tx/tests/router_tests.rs`
- Modify: `programs/supersonic-tx/Cargo.toml` `[dev-dependencies]`
- Keep weak inline unit tests only as supplements — primary coverage is program-test

**Interfaces:**
- Consumes: program BPF/rlib with `no-entrypoint` for host tests as required by Anchor
- Produces: three tests:
  1. `noop_decoy` succeeds
  2. `execute_fuzzy_bundle` with `decoy_count == 0` errors `InvalidBundleManifest`
  3. CPI happy path via remaining accounts (system transfer or trivial executable)

- [ ] **Step 1: Write failing program tests**

Rewrite `programs/supersonic-tx/tests/router_tests.rs` to use `solana_program_test` / Anchor test patterns. Minimal sketch:

```rust
use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[tokio::test]
async fn noop_decoy_succeeds() {
    let program_id = supersonic_tx_core::program_id();
    let mut program_test = ProgramTest::new(
        "supersonic_tx",
        program_id,
        processor!(supersonic_tx::entry),
    );
    // If entrypoint export differs under Anchor, use anchor's preferred ProgramTest setup
    // documented for 0.30.1 — adjust processor! binding to match built program.
    let (mut banks, payer, hash) = program_test.start().await;
    let ix = /* build noop_decoy ix with discriminator + entropy_seed */;
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], hash);
    banks.process_transaction(tx).await.expect("noop_decoy should succeed");
}

#[tokio::test]
async fn execute_fuzzy_bundle_rejects_zero_decoys() {
    // build execute_fuzzy_bundle with decoy_count = 0
    // expect Err containing InvalidBundleManifest
}

#[tokio::test]
async fn execute_fuzzy_bundle_cpi_system_transfer() {
    // remaining_accounts[0] = system program executable account meta pattern per lib.rs
    // invoke transfer via instruction_data
}
```

Implementers: prefer Anchor's `#[tokio::test]` + `anchor_lang::prelude::*` patterns already used in ecosystem for 0.30.1; if `processor!(supersonic_tx::entry)` is awkward, use `anchor_client`/`ProgramTest::add_program` with built `.so` from `target/deploy/supersonic_tx.so` after `anchor build`.

- [ ] **Step 2: Run fail**

```bash
cargo test -p supersonic-tx --test router_tests -- --nocapture
```

Expected: FAIL until wired.

- [ ] **Step 3: Implement / wire program test harness**

Add to `programs/supersonic-tx/Cargo.toml`:

```toml
[dev-dependencies]
solana-program-test = "~1.18"
solana-sdk = { workspace = true }
tokio = { workspace = true }
supersonic-tx-core = { path = "../../crates/supersonic-tx-core" }
```

Ensure features allow host testing (`cpi`, `no-entrypoint` as needed). Delete the old “assert 0==0” style tests in `router_tests.rs` (inline `lib.rs` event struct tests may remain).

- [ ] **Step 4: Run pass + anchor build**

```bash
anchor build
cargo test -p supersonic-tx --test router_tests -- --nocapture
```

Expected: both succeed.

- [ ] **Step 5: Commit**

```bash
git add programs/supersonic-tx
git commit -m "test: add solana-program-test coverage for router instructions"
```

---

### Task 12: Deploy path documentation

**Files:**
- Create: `docs/deploy.md`
- Modify: `README.md` (link Deployments section stub — full rewrite in Task 18)

**Interfaces:**
- Consumes: Task 3 program id
- Produces: operator steps — no code change required beyond docs

- [ ] **Step 1: Write deploy doc with exact commands**

`docs/deploy.md`:

```markdown
# Deploying `supersonic_tx`

1. Sync IDs: `declare_id!`, `supersonic_tx_core::PROGRAM_ID_STR`, `Anchor.toml` must match
   `solana-keygen pubkey target/deploy/supersonic_tx-keypair.json`.
2. `anchor build`
3. `solana airdrop 2` (devnet) to provider wallet if needed
4. `anchor deploy --provider.cluster devnet`
5. Record program id under README Deployments
6. Smoke: `supersonic-tx cook ...` → `simulate` → `cast ... --send` on devnet
7. Mainnet is optional and explicit — not required for v1 bar C
```

- [ ] **Step 2: Verify commands are copy-pasteable**

Manually read file; ensure no TBD.

- [ ] **Step 3: (No product code)**

- [ ] **Step 4: Confirm `anchor build` still works**

```bash
anchor build
```

- [ ] **Step 5: Commit**

```bash
git add docs/deploy.md
git commit -m "docs: add devnet deploy path for supersonic_tx router"
```

---

# Milestone M4 — ALT + cast/simulate honesty

### Task 13: `AltResolver` real RPC fetch + fallback

**Files:**
- Create: `crates/supersonic-tx-sdk/src/alt.rs`
- Modify: `crates/supersonic-tx-sdk/src/lib.rs`
- Modify: `crates/supersonic-tx-core/src/types.rs` (`AltFetchFailed` optional)
- Test: `alt.rs` with mocked account data decode unit test

**Interfaces:**
- Consumes: `RpcClient`, ALT `Pubkey`
- Produces:

```rust
pub struct AltResolver;

impl AltResolver {
    pub async fn fetch(
        rpc: &solana_client::nonblocking::rpc_client::RpcClient,
        alt: &Pubkey,
    ) -> Result<AddressLookupTableAccount, SupersonicError>;
}
```

On failure: caller logs warning and continues with `&[]` (non-ALT V0) + aggressive shrink (CLI Task 15).

Decode using `solana_sdk::address_lookup_table::state::AddressLookupTable::deserialize` (1.18 path).

- [ ] **Step 1: Write failing unit test for deserialize helper**

```rust
#[test]
fn rejects_empty_account_data() {
    let err = AltResolver::from_account_data(&Pubkey::new_unique(), &[]).unwrap_err();
    let _ = err; // expect Alt/InvalidDecoyConfig
}
```

```rust
pub fn from_account_data(key: &Pubkey, data: &[u8]) -> Result<AddressLookupTableAccount, SupersonicError>;
```

- [ ] **Step 2: Run fail**

```bash
cargo test -p supersonic-tx-sdk rejects_empty_account_data -- --nocapture
```

- [ ] **Step 3: Implement `alt.rs`**

```rust
pub async fn fetch(...) -> Result<AddressLookupTableAccount, SupersonicError> {
    let acc = rpc.get_account(alt).await.map_err(|e| SupersonicError::RouterError(e.to_string()))?;
    Self::from_account_data(alt, &acc.data)
}
```

Export module from `lib.rs`.

- [ ] **Step 4: Run pass**

```bash
cargo test -p supersonic-tx-sdk rejects_empty_account_data -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-sdk/src/alt.rs crates/supersonic-tx-sdk/src/lib.rs crates/supersonic-tx-core/src/types.rs
git commit -m "feat: AltResolver RPC fetch for real lookup tables"
```

---

### Task 14: `sign_versioned_tx` + `simulate_and_send` (`--send` gate)

**Files:**
- Create: `crates/supersonic-tx-sdk/src/sign.rs`
- Modify: `crates/supersonic-tx-sdk/src/lib.rs`
- Modify: `crates/supersonic-tx-core/src/types.rs` add `MissingSignature`, `SimulationFailed`, `BroadcastDisabled`
- Test: `sign.rs`

**Interfaces:**
- Consumes: `VersionedMessage`, `&[&Keypair]` (or `Vec<&dyn Signer>`)
- Produces:

```rust
pub fn sign_versioned_tx(
    message: VersionedMessage,
    signers: &[&Keypair],
) -> Result<VersionedTransaction, SupersonicError> {
    VersionedTransaction::try_new(message, signers).map_err(|e| {
        SupersonicError::MissingSignature(e.to_string())
    })
}

pub struct SendOptions {
    pub broadcast: bool, // maps from CLI --send
}

pub async fn simulate_and_send(
    rpc: &RpcClient,
    tx: &VersionedTransaction,
    opts: SendOptions,
) -> Result<Option<Signature>, SupersonicError>;
```

Behavior: always `simulate_transaction`; if sim fails → error; if `broadcast == false` → return `Ok(None)` and do not send; if true → `send_transaction` → `Ok(Some(sig))`. **Never** send when any signature is default.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn sign_versioned_tx_rejects_missing_signer() {
    let payer = Keypair::new();
    let ix = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1);
    let msg = FuzzyBundleBuilder::compile_v0_message(
        &payer.pubkey(),
        &[ix],
        &[],
        Hash::new_unique(),
    ).unwrap();
    let err = sign_versioned_tx(msg, &[]).unwrap_err();
    assert!(matches!(err, SupersonicError::MissingSignature(_)));
}

#[test]
fn signed_tx_has_no_default_signatures() {
    let payer = Keypair::new();
    let ix = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1);
    let msg = FuzzyBundleBuilder::compile_v0_message(
        &payer.pubkey(),
        &[ix],
        &[],
        Hash::new_unique(),
    ).unwrap();
    let tx = sign_versioned_tx(msg, &[&payer]).unwrap();
    assert!(tx.signatures.iter().all(|s| *s != Signature::default()));
}
```

Add `MissingSignature(String)` to `SupersonicError`.

- [ ] **Step 2: Run fail**

```bash
cargo test -p supersonic-tx-sdk sign_versioned_tx_rejects_missing_signer signed_tx_has_no_default_signatures -- --nocapture
```

- [ ] **Step 3: Implement `sign.rs`**

Also add helper `fn assert_fully_signed(tx: &VersionedTransaction) -> Result<(), SupersonicError>`.

- [ ] **Step 4: Run pass**

```bash
cargo test -p supersonic-tx-sdk -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-sdk/src/sign.rs crates/supersonic-tx-sdk/src/lib.rs crates/supersonic-tx-core/src/types.rs
git commit -m "feat: sign VersionedTransaction and gate broadcast behind SendOptions"
```

---

### Task 15: CLI `simulate` / `cast` honesty (`--handoff`, `--send`, real ALT)

**Files:**
- Modify: `crates/supersonic-tx-cli/src/main.rs`
- Test: clap parse tests for new flags

**Interfaces:**
- Consumes: handoff loader, `AltResolver`, `sign_versioned_tx`, `simulate_and_send`, cooker `assert_funded_for_cast`
- Produces: updated CLI contract per spec §15

`cast` flags: `--target`, `--amount`, `--level`, `--rpc-url`, `--handoff` XOR `--keypair`, `--alt`, `--send`, `--tip` (repeatable), `--via-router` (opt-in CPI; default off)

Behavior:
1. Load fee payer + sinks from handoff or keypair
2. `assert_funded_for_cast` when handoff present
3. Build sinks into `FuzzyBundleBuilder::with_sinks`
4. If `--alt`: `AltResolver::fetch`; on err warn + empty ALTs
5. `get_latest_blockhash` → `build_versioned_message` → `sign_versioned_tx`
6. `simulate_and_send(..., SendOptions { broadcast: send_flag })`
7. Remove fake in-memory ALT construction (`AddressLookupTableAccount { addresses: vec![target, payer] }`)
8. Remove broadcast of dummy-signature txs

`simulate`: never broadcasts; prints decoy ratio, CU, MTU fill, Benford status; may assemble without real keys.

- [ ] **Step 1: Write failing clap tests**

```rust
#[test]
fn test_cli_parse_cast_send_and_handoff() {
    let args = vec![
        "supersonic-tx", "cast",
        "--target", "4vMGoEDFfVJjF9y85sSvh4WwP76d9B54tE86b8xXN6T",
        "--handoff", "/tmp/cook/handoff-1.json",
        "--send",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Cast { handoff, send, .. } => {
            assert_eq!(handoff.as_deref(), Some("/tmp/cook/handoff-1.json"));
            assert!(send);
        }
        _ => panic!("cast"),
    }
}
```

Update existing cast parse tests to include new fields with defaults (`send: false`, `handoff: None`).

- [ ] **Step 2: Run fail**

```bash
cargo test -p supersonic-tx-cli test_cli_parse_cast_send_and_handoff -- --nocapture
```

- [ ] **Step 3: Implement CLI wiring**

Delete the comment block that admits dummy signatures. Default: no send. Print clear line when broadcast skipped: `Broadcast skipped (pass --send to submit)`.

- [ ] **Step 4: Run pass**

```bash
cargo test -p supersonic-tx-cli -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-cli/src/main.rs
git commit -m "feat: honest cast/simulate with handoff, ALT fetch, and --send gate"
```

---

# Milestone M5 — Campaign + ship

### Task 16: `CampaignPlan` planner (isolate-intent default)

**Files:**
- Create: `crates/supersonic-tx-sdk/src/campaign.rs`
- Modify: `crates/supersonic-tx-sdk/src/lib.rs`
- Test: `campaign.rs`

**Interfaces:**
- Consumes: target ix(s), sinks, level, `isolate_intent: bool`, `decoy_tx_count: usize`
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedTxKind { DecoyOnly, RealIntent, PostNoise }

#[derive(Debug, Clone)]
pub struct PlannedTx {
    pub kind: PlannedTxKind,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct CampaignPlan {
    pub txs: Vec<PlannedTx>,
}

pub struct CampaignPlanner { /* fields */ }

impl CampaignPlanner {
    pub fn new(payer: Pubkey, level: ObfuscationLevel) -> Self;
    pub fn with_sinks(self, sinks: Vec<Pubkey>) -> Self;
    pub fn isolate_intent(self, yes: bool) -> Self; // default true at CLI
    pub fn decoy_tx_count(self, n: usize) -> Self;
    pub fn plan(self, target_ixs: Vec<Instruction>) -> Result<CampaignPlan, SupersonicError>;
}
```

Rules:
- Decoy-only txs: tips/self-transfers/router noops/CU/memo — **never** include target ixs
- Real-intent tx when `isolate_intent`: target + only known-safe Light padding (CU/memo/noop) — **no** statistical transfers
- When `isolate_intent == false`: allow mixing (document risk); still fail-soft only
- Execution semantics (CLI Task 17): decoy txs best-effort; real intent hard-fail

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn isolated_plan_keeps_target_out_of_decoy_txs() {
    let payer = Pubkey::new_unique();
    let sink = Pubkey::new_unique();
    let target = system_instruction::transfer(&payer, &Pubkey::new_unique(), 42);
    let plan = CampaignPlanner::new(payer, ObfuscationLevel::Standard)
        .with_sinks(vec![sink])
        .isolate_intent(true)
        .decoy_tx_count(2)
        .plan(vec![target.clone()])
        .unwrap();
    let decoys: Vec<_> = plan.txs.iter().filter(|t| t.kind == PlannedTxKind::DecoyOnly).collect();
    assert_eq!(decoys.len(), 2);
    for d in decoys {
        assert!(d.instructions.iter().all(|ix| ix != &target));
    }
    let real = plan.txs.iter().find(|t| t.kind == PlannedTxKind::RealIntent).unwrap();
    assert!(real.instructions.iter().any(|ix| ix == &target));
    assert!(real.instructions.iter().all(|ix| {
        ix == &target
            || ix.program_id == solana_sdk::compute_budget::id()
            || ix.program_id.to_string() == "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"
            || ix.program_id == program_id()
    }));
}
```

- [ ] **Step 2: Run fail**

```bash
cargo test -p supersonic-tx-sdk isolated_plan_keeps_target_out_of_decoy_txs -- --nocapture
```

- [ ] **Step 3: Implement planner**

- [ ] **Step 4: Run pass**

```bash
cargo test -p supersonic-tx-sdk isolated_plan_keeps_target_out_of_decoy_txs -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-sdk/src/campaign.rs crates/supersonic-tx-sdk/src/lib.rs
git commit -m "feat: CampaignPlanner with isolate-intent decoy separation"
```

---

### Task 17: CLI `campaign` subcommand

**Files:**
- Modify: `crates/supersonic-tx-cli/src/main.rs`
- Test: clap + isolation default

**Interfaces:**
- Consumes: `CampaignPlan`, sign/send helpers
- Produces:

```text
supersonic-tx campaign \
  --target <PK> --amount <u64> --level standard \
  --rpc-url <URL> --handoff <PATH> \
  --txs <N> \
  --isolate-intent true|false \  # default true
  --send \
  [--alt <PK>] [--drain]
```

Loop:
1. For each `DecoyOnly` / `PostNoise`: sign → simulate → send if `--send`; on failure log + continue
2. For `RealIntent`: sign → simulate → send if `--send`; on failure non-zero exit
3. Optional `--drain` calls cooker drain

- [ ] **Step 1: Write failing parse test**

```rust
#[test]
fn test_cli_parse_campaign_defaults_isolate_true() {
    let args = vec![
        "supersonic-tx", "campaign",
        "--target", "4vMGoEDFfVJjF9y85sSvh4WwP76d9B54tE86b8xXN6T",
        "--handoff", "/tmp/h.json",
        "--txs", "3",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Campaign { isolate_intent, send, txs, .. } => {
            assert!(isolate_intent);
            assert!(!send);
            assert_eq!(txs, 3);
        }
        _ => panic!("campaign"),
    }
}
```

Use `#[arg(long, default_value_t = true, action = clap::ArgAction::Set)] isolate_intent: bool` or `default_value = "true"` with bool parser.

- [ ] **Step 2: Run fail**

```bash
cargo test -p supersonic-tx-cli test_cli_parse_campaign_defaults_isolate_true -- --nocapture
```

- [ ] **Step 3: Implement handler**

- [ ] **Step 4: Run pass**

```bash
cargo test -p supersonic-tx-cli -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-cli/src/main.rs
git commit -m "feat: CLI campaign with isolate-intent default and fail-soft decoys"
```

---

### Task 18: CI workflow + commit `Cargo.lock`

**Files:**
- Create: `.github/workflows/ci.yml`
- Create/update: `Cargo.lock` (via `cargo generate-lockfile` / build)
- Modify: `.gitignore` to ensure `Cargo.lock` is **not** ignored for this binary workspace

**Interfaces:**
- Consumes: green local `cargo test --workspace` + `anchor build`
- Produces: CI jobs `rust` and `anchor`

- [ ] **Step 1: Write workflow file (red until CI runs)**

`.github/workflows/ci.yml`:

```yaml
name: ci
on:
  push:
  pull_request:

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Test workspace
        run: cargo test --workspace

  anchor:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Solana
        run: |
          sh -c "$(curl -sSfL https://release.anza.xyz/v1.18.26/install)"
          echo "$HOME/.local/share/solana/install/active_release/bin" >> $GITHUB_PATH
      - name: Install Anchor 0.30.1
        run: cargo install --git https://github.com/coral-xyz/anchor --tag v0.30.1 anchor-cli --locked
      - name: Anchor build
        run: anchor build
```

(Adjust Solana install URL if Anza path differs in environment; keep versions aligned to `~1.18` / Anchor `0.30.1`.)

- [ ] **Step 2: Generate lockfile locally**

```bash
cargo generate-lockfile
cargo test --workspace
anchor build
```

Expected: PASS locally before claiming CI-ready.

- [ ] **Step 3: Ensure lockfile tracked**

```bash
git status -- Cargo.lock
# must show as new/modified file to add, not ignored
```

- [ ] **Step 4: Re-run workspace tests**

```bash
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml Cargo.lock .gitignore
git commit -m "ci: workspace cargo test and anchor build; commit Cargo.lock"
```

---

### Task 19: README + ARCHITECTURE rewrite (honest threat model)

**Files:**
- Modify: `README.md`
- Modify: `ARCHITECTURE.md`
- Modify: `info` subcommand output in `crates/supersonic-tx-cli/src/main.rs` (limits matrix)

**Interfaces:**
- Consumes: spec §§2, 7, 9, 15
- Produces: docs without fake Jupiter decoys; explicit “does not stop” list; CLI `info` lists allowed decoy kinds; Deployments section; no `.agents/` victory claims

- [ ] **Step 1: Write failing doc grep gate**

```bash
rg -n "Jupiter router|Raydium pools|Orca vaults|victory|bounty complete" README.md ARCHITECTURE.md || true
```

Treat remaining Jupiter-as-decoy narrative as failure to fix.

- [ ] **Step 2: Confirm current docs still sell forbidden decoys**

Expected: ARCHITECTURE.md still mentions Jupiter/Raydium/Orca as decoy targets.

- [ ] **Step 3: Rewrite docs**

README must include:
- Mission: behavioral obscurity, not mixing/ZK
- Threat table: helps vs does not stop (sponsor edge, timing, human review, shared router filter)
- Workspace layout including `crates/account-cooker`
- CLI table: cook / simulate / cast / campaign / info
- Note `--send` opt-in; campaign `--isolate-intent` default true
- Link `docs/deploy.md`
- Allowed decoys: tip/sink SOL transfers, self-transfers among cooked accounts, CU, memo, router `noop_decoy`

ARCHITECTURE must diagram Approach 1 and remove fake DEX destination strategy.

Update `Commands::Info` printout accordingly (allowed kinds + honest limits).

- [ ] **Step 4: Re-grep**

```bash
rg -n "JUP6Lkb|fake Jupiter|Raydium pools as decoy" README.md ARCHITECTURE.md crates/supersonic-tx-cli/src/main.rs
cargo test -p supersonic-tx-cli test_cli_parse_info -- --nocapture
```

Expected: clean; info still parses.

- [ ] **Step 5: Commit**

```bash
git add README.md ARCHITECTURE.md crates/supersonic-tx-cli/src/main.rs
git commit -m "docs: honest threat model and fail-soft decoy narrative"
```

---

### Task 20: End-to-end bar C verification checklist

**Files:**
- None new (verification only); optional `docs/superpowers/plans/` checkbox updates

**Interfaces:**
- Consumes: all prior tasks
- Produces: evidence that bar C exit criteria hold

- [ ] **Step 1: Write the verification checklist as commands (run them)**

```bash
rg -n "Super111|Signature::default\(\)|JUP6Lkb|INCLUDINGRemind" \
  programs crates LICENSE-MIT Anchor.toml || true
cargo test --workspace
anchor build
test -f .github/workflows/ci.yml && test -f Cargo.lock
test -d crates/account-cooker
rg -n "isolate_intent|Commands::Campaign|Commands::Cook" crates/supersonic-tx-cli/src/main.rs
rg -n "AltResolver|sign_versioned_tx|CampaignPlanner" crates/supersonic-tx-sdk/src/*.rs
```

- [ ] **Step 2: Expect zero forbidden patterns in product paths**

`Signature::default()` may remain **only** inside `estimate_tx_size` — annotate with comment `// size estimate slots only — never broadcast`. Grep send paths to ensure CLI/SDK send helpers never construct default sigs for RPC.

- [ ] **Step 3: Fix any remaining gaps discovered**

If gaps appear, open a follow-up commit per gap (do not leave TBD).

- [ ] **Step 4: Final green run**

```bash
cargo test --workspace && anchor build
```

Expected: PASS.

- [ ] **Step 5: Commit** (only if Step 3 produced fixes)

```bash
git add -A
git commit -m "chore: bar C verification fixes"
```

If no fixes: skip empty commit.

---

## Stretch (optional, not required for v1 done)

### Task S1: Optional `Submitter` trait (Jito behind feature)

Only if clean: `RpcSubmitter` default + `JitoSubmitter` behind `feature = "jito"`. Do **not** block bar C. Skip unless explicitly requested after Task 20.

---

## Self-Review Record

### 1. Spec coverage map

| Spec section | Tasks |
| --- | --- |
| §2 Goals / §3 Approach 1 | Header + all milestones |
| §4 Workspace / types / handoff / SDK surface | Tasks 3–10, 13–17 |
| §5 Cast + campaign flows | Tasks 15–17 |
| §6 Program + events + tests | Tasks 3, 11–12 |
| §7 Fail-soft decoys / ALT / MTU / signing | Tasks 4–6, 13–15 |
| §8 account-cooker | Tasks 7–10 |
| §9 Errors / threat model docs | Tasks 14, 19 |
| §10 Testing strategy | Tasks throughout (TDD) + 11, 18, 20 |
| §11 CI / license / agents / lockfile / deploy | Tasks 1, 12, 18–20 |
| §12 Gap closure table | Tasks 2–6, 11, 13–15, 18–19 |
| §13 Jito stretch | Task S1 optional |
| §14 Milestones M0–M5 | Plan milestones M0–M5 |
| §15 CLI contract | Tasks 10, 15, 17, 19 |
| §16 Resolved ambiguities | Global Constraints + Tasks 14–17 |

**Gaps after review:** none intentional. Sibling empty `../account-cooker/` is noted as non-authoritative; real crate is `crates/account-cooker/`.

### 2. Placeholder scan

Plan scanned for TBD/TODO/`similar to Task N`/vague “add validation” — none left as open implementer work. Program-test harness notes allow Anchor-0.30.1 binding adjustments without leaving product behavior unspecified.

### 3. Type/name consistency

Canonical names used across tasks: `HandoffBundle`, `CookedAccount`, `CookedRole`, `Cooker`, `CookerConfig`, `CookerError`, `FuzzyBundleBuilder::build_versioned_message`, `compile_v0_message`, `estimate_tx_size`, `AltResolver`, `sign_versioned_tx`, `simulate_and_send`, `SendOptions { broadcast }`, `CampaignPlanner`, `CampaignPlan`, `PlannedTx`, `PlannedTxKind`, `PROGRAM_ID_STR` / `program_id()`, `ObfuscationLevel`, `MAX_TX_PAYLOAD_BYTES`, CLI `--send`, `--isolate-intent` default true, `--handoff`, `--via-router` opt-in.

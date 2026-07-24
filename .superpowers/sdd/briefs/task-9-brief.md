# Task 9 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- account-cooker at `crates/account-cooker/`
- Handoff schema_version=1; relative secret_key_path
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes

- Branch `feature/bar-c`; HEAD `050c203`
- Tasks 1–8 complete
- A parallel prod-bugfix may be editing other crates — prefer cooker-focused changes; resolve conflicts carefully
- WDAC may block cargo
- Do not commit secrets/keypairs/`.agents`

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

Reuse: if `keys/*.json` already exists for same pubkey path or duplicate pubkeys in one handoff â†’ push into `warnings` and print path for CLI.

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

`fund_accounts`: for each cooked account, `system_instruction::transfer` from sponsor for `funded_lamports`, sign+send (skip in unit tests â€” gate RPC methods behind integration or test with `solana-test-validator` documented in README). Unit coverage focuses on assert/reuse.

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


# Task 8 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- `account-cooker` at `crates/account-cooker/`
- Handoff `schema_version = 1`; relative `secret_key_path` only
- Fail-soft decoys; TrustedSystemAccount only via tip/cooker paths
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes

- Branch `feature/bar-c`; HEAD `28d4d9b`
- Tasks 1–7 complete (Task 5 sealed; Task 7 validation approved)
- Minors for final: Task 4 CLI diag; Task 6 FromStr/asserts; Task 7 cluster enum
- WDAC may block cargo; deliver correct source + tests
- Do not commit keypairs/`.agents`/secrets

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


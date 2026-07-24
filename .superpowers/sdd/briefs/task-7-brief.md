# Task 7 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- `account-cooker` crate at `crates/account-cooker/` (NOT sibling empty stub)
- Handoff JSON `schema_version = 1`: `secret_key_path` relative paths only — no embedded secrets
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes

- Branch `feature/bar-c`; HEAD `a11151e`
- Tasks 1–6 complete; Task 4 minors + Task 6 minors for final review
- WDAC may block cargo; still deliver compiling source + tests
- Do not commit keypairs/`.agents`/secrets

---
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

Scaffold crate so test compiles against missing types (will fail compile first â€” that's the red).

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


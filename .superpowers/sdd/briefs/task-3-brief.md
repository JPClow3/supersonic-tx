# Task 3 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- Pins: Anchor 0.30.1, Solana ~1.18, MIT, binary `supersonic-tx`
- Real program ID required (no Super111 placeholder)
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes from prior tasks

- Branch: `feature/bar-c`; HEAD after Task 2: `6b7e922`
- Many workspace sources may still be **untracked** (Task 1 only committed hygiene; Task 2 only Cargo.toml). When you touch files, `git add` the real on-disk sources for members you need so the repo becomes self-contained. Do not commit `.agents/`, keypairs, or secrets.
- Host may block `cargo check` build scripts (WDAC 4551). Prefer `solana-keygen` if available; otherwise generate a valid keypair another reliable way. Record WDAC limitations in the report if compile proof fails.

---
### Task 3: Real program ID plumbing (local keypair + synced constants)

**Files:**
- Create: `target/deploy/supersonic_tx-keypair.json` (generate; do **not** commit keypair â€” keep gitignored)
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


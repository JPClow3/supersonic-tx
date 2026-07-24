# Task 4 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- V0 compile MUST use Solana 1.18 API: `solana_sdk::message::v0::Message::try_compile` then `VersionedMessage::V0(...)` — do **not** call nonexistent `VersionedMessage::try_compile`
- `MAX_TX_PAYLOAD_BYTES = 1232`; serialize size checks with `bincode`
- Pins: Anchor 0.30.1, Solana ~1.18
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes

- Branch `feature/bar-c`; HEAD `67fed6c`
- Program ID now `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`
- WDAC may block cargo build scripts (4551). Still make the API fix; document if check cannot complete.
- Do not commit keypairs, `.agents/`, secrets.

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
  - `FuzzyBundleBuilder::build_versioned_message(...)` shrink loop returning `VersionedMessage` (not a falsely â€œready to sendâ€ signed tx)
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

Keep a thin deprecated wrapper if CLI still calls `build_versioned_transaction` â€” change it to return unsigned estimation-only OR remove and update call sites in same milestone (prefer remove/replace with `build_versioned_message` + later `sign_versioned_tx`).

Implement `shrink_decoys` priority (spec Â§7.4): drop statistical transfers first, then memo, then extra router noops; never drop all CU padding if a CU ix exists; never drop target ixs. Heuristic: classify by `program_id` (system transfer = statistical, memo program, router `PROGRAM_ID`, compute budget).

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


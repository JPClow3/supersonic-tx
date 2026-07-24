# Task 6 Brief — MTU shrink-priority unit tests

## Global Constraints

- Workspace root for all product work: `H:\Code\Pessoais\SP - Solana\supersonic-tx\` (sibling empty stub `..\account-cooker\` is not the crate path — implement at `crates/account-cooker/`).
- Pins: Anchor `0.30.1`, Solana crates `~1.18`, MIT license, binary name `supersonic-tx`.
- V0 compile must use Solana 1.18 API: `solana_sdk::message::v0::Message::try_compile` → `VersionedMessage::V0(...)` (do not call nonexistent `VersionedMessage::try_compile`).
- `MAX_TX_PAYLOAD_BYTES = 1232`; serialize size checks with `bincode`.
- Fail-soft decoys only; never fake Jupiter/DEX destinations; never transfer to executable program IDs; never broadcast `Signature::default()`.
- Handoff JSON `schema_version = 1`: `secret_key_path` relative paths only — no embedded secrets in v1.
- CLI: `--send` opt-in for broadcast; `campaign --isolate-intent` default **true**; default RPC `https://api.devnet.solana.com`.
- Jito / ephemeral programs / SPL decoy graphs: out of scope for v1 done.
- Spec authority: `docs/superpowers/specs/2026-07-23-supersonic-tx-design.md`.
- Exit bar C: `cargo test --workspace` + `anchor build` green; cooker + campaign + ALT + CI + documented deploy path.

## Scope

**Files:**
- Modify: `crates/supersonic-tx-sdk/src/builder.rs` tests
- Test: same

**Interfaces:**
- Consumes `shrink_decoys` from Task 4.
- Produces documented priority — statistical → memo → extra noop; retain ≥1 CU when present.

## Implementation

Expose `shrink_decoys` as `pub(crate) fn shrink_decoys` and add a `#[cfg(test)] pub fn shrink_decoys_for_test` wrapper.

Add this test:

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
                || ix.data.first() != Some(&2))
    );
    assert_eq!(manifest.decoy_instructions.len(), 2);
}
```

Run:

```text
cargo test -p supersonic-tx-sdk shrink_drops_statistical_before_memo -- --nocapture
```

Expected: pass. Commit:

```text
git add crates/supersonic-tx-sdk/src/builder.rs
git commit -m "test: enforce MTU shrink priority for fail-soft decoys"
```

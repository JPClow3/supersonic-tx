# Task 11 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- Real program tests via solana-program-test
- Critical C3 from bug hunt: CPI must NOT emit success without executing CPI — reject empty/non-executable remaining_accounts before success events
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes

- Branch `feature/bar-c`; HEAD `7e41014`
- Tasks 1–10 complete
- Bug hunt C3 mapped to this task — fix dishonest CPI success if present in lib.rs
- WDAC may block cargo/program-test; still write correct tests + program guards
- Do not commit secrets

---
### Task 11: `solana-program-test` for noop, decoy_count==0, CPI path

**Files:**
- Replace: `programs/supersonic-tx/tests/router_tests.rs`
- Modify: `programs/supersonic-tx/Cargo.toml` `[dev-dependencies]`
- Keep weak inline unit tests only as supplements â€” primary coverage is program-test

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
    // documented for 0.30.1 â€” adjust processor! binding to match built program.
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

Ensure features allow host testing (`cpi`, `no-entrypoint` as needed). Delete the old â€œassert 0==0â€ style tests in `router_tests.rs` (inline `lib.rs` event struct tests may remain).

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


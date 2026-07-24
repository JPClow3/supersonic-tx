# Task 5 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- Fail-soft decoys only; never fake Jupiter/DEX destinations; never transfer to executable program IDs
- Tip destinations: config allowlist + cooked sinks
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes

- Branch `feature/bar-c`; HEAD `608c459`
- Minors for final review (Task 4): CLI diagnostic manifest mismatch; missing runtime `--send` refusal test
- WDAC may block cargo; document if needed
- Do not commit keypairs/`.agents`/secrets

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
  - `StatisticalTransferNoise::default_tip_allowlist()` â€” **real tip/fee receivers or empty requiring injection**; **remove** bogus Jupiter key `JUP6LkbZbjS1jKKwapdH67yN5k8u4nKq1X4fD6F9yM5`
  - Level counts for `AnchorRouterNoise` per spec: Light `0..=1`, Standard `1`, Paranoid `1..=2` (use fixed Standard=1; Light random 0â€“1 or 1; Paranoid 2)
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
        // without RPC â€” enforce allowlist membership instead in builder tests.
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
    ObfuscationLevel::Light => 1,      // within 0â€“1; use 1 for demo density
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


# Task 2 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- Pins: Anchor 0.30.1, Solana ~1.18, MIT, binary `supersonic-tx`
- MAX_TX_PAYLOAD_BYTES = 1232; serialize size checks with `bincode`
- V0 compile: `solana_sdk::message::v0::Message::try_compile` then `VersionedMessage::V0(...)`
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

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

Expected: no â€œcan't find crate `bincode`â€; `try_compile` / other errors may remain until Task 4.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/supersonic-tx-sdk/Cargo.toml crates/supersonic-tx-cli/Cargo.toml
git commit -m "build: add workspace bincode for SDK and CLI"
```

---

# Milestone M1 â€” Core truthfulness


# Task 12 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- Documented deploy path; program ID already `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes

- Branch `feature/bar-c`; HEAD `7845e55`
- Tasks 1–11 complete; C3 CPI honesty done
- Do not commit deploy keypairs
- WDAC may block anchor build — still write docs/scripts correctly

---
### Task 12: Deploy path documentation

**Files:**
- Create: `docs/deploy.md`
- Modify: `README.md` (link Deployments section stub â€” full rewrite in Task 18)

**Interfaces:**
- Consumes: Task 3 program id
- Produces: operator steps â€” no code change required beyond docs

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
6. Smoke: `supersonic-tx cook ...` â†’ `simulate` â†’ `cast ... --send` on devnet
7. Mainnet is optional and explicit â€” not required for v1 bar C
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

# Milestone M4 â€” ALT + cast/simulate honesty


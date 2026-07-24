# Task 1 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- Sibling empty stub `..\account-cooker\` is NOT the crate path
- Pins: Anchor 0.30.1, Solana ~1.18, MIT, binary `supersonic-tx`
- Fail-soft decoys only; no fake DEX IDs; never Signature::default()
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

---
### Task 1: License typo, gitignore, agents quarantine

**Files:**
- Modify: `LICENSE-MIT`
- Create: `.gitignore`
- Create: `docs/internal/agents-archive/README.md` (if relocating) **or** only ignore `.agents/`
- Optional: move `.agents/` â†’ `docs/internal/agents-archive/.agents/`

**Interfaces:**
- Consumes: none
- Produces: corrected MIT text; `.agents/` excluded from product narrative / git tracking

- [ ] **Step 1: Write a failing hygiene check script assertion (document as test)**

Create `scripts/check_hygiene.sh` is optional; prefer a tiny Rust-free verification by grepping. For TDD without a harness, add a CLI unit later â€” for this task use shell checks as the gate.

- [ ] **Step 2: Run check to verify current failure**

Run from `supersonic-tx/`:

```bash
rg -n "INCLUDINGRemind" LICENSE-MIT
```

Expected: match on line with `INCLUDINGRemind`.

- [ ] **Step 3: Fix LICENSE and add `.gitignore`**

In `LICENSE-MIT`, replace:

```text
IMPLIED, INCLUDINGRemind BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
```

with:

```text
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
```

Create `.gitignore`:

```gitignore
/target/
**/*.rs.bk
.anchor/
.DS_Store
*.swp
# Deploy & operator secrets
target/deploy/*-keypair.json
**/keys/
**/handoff-*.json
!docs/**
# Agent process notes are non-authoritative (spec Â§11.1)
.agents/
```

Create `docs/internal/agents-archive/README.md`:

```markdown
# Agents archive (non-authoritative)

Files under `.agents/` (ignored) or archived here are **process notes from agent runs**.
They are **not** proof of bounty completion. Source of truth: `cargo test --workspace`,
`anchor build`, and this repository's product docs / CI.
```

Optionally move the existing `.agents/` tree into `docs/internal/agents-archive/` then keep `.agents/` in gitignore for future local notes. Product README must not link to them as victory proof.

- [ ] **Step 4: Re-run checks**

```bash
rg -n "INCLUDINGRemind" LICENSE-MIT; echo "exit:$?"
# expect no matches
test -f .gitignore && echo "gitignore ok"
```

Expected: no `INCLUDINGRemind`; gitignore present.

- [ ] **Step 5: Commit**

```bash
git add LICENSE-MIT .gitignore docs/internal/agents-archive/README.md
git commit -m "chore: fix MIT typo and quarantine agent notes"
```

---


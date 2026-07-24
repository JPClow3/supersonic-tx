# Task 12 Report — Deploy path documentation

**Status:** DONE_WITH_CONCERNS
**Branch:** `feature/bar-c`
**Commit:** `19497ef` — `docs: add devnet deploy path for supersonic_tx router`
**Date:** 2026-07-23

---

## Summary

Replaced `docs/deploy.md` with the brief’s verbatim operator steps (ID sync, `anchor build`, devnet airdrop/deploy, README Deployments recording, smoke flow, optional mainnet note). Added a README **Deployments** section stub linking to the deploy doc. No product code or keypairs committed.

---

## Brief steps

| Step | Result |
|------|--------|
| 1. Write deploy doc with exact commands | `docs/deploy.md` matches brief; no TBD |
| 2. Verify copy-pasteable | Manual read passed |
| 3. No product code | None changed |
| 4. `anchor build` | Not run — `anchor` absent from PATH on this host |
| 5. Commit | `19497ef` on `feature/bar-c` |

---

## Concerns

- `anchor build` could not be executed locally (`anchor` not in PATH). CI Linux gate remains the authoritative build check per README.
- Program ID `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` is synced in source; no cluster deployment is recorded yet.

---

## Files

- `docs/deploy.md` — created/updated per brief
- `README.md` — added `## Deployments` stub with link to deploy doc

# Task 2 Report — Workspace `bincode` pin and SDK dependency wire-up

**Status:** DONE_WITH_CONCERNS  
**Branch:** `feature/bar-c`  
**Commit:** `6b7e922` — `build: add workspace bincode for SDK and CLI`  
**Date:** 2026-07-23

---

## Summary

Pinned `bincode = "1.3"` in the workspace root and wired `bincode.workspace = true` into both `supersonic-tx-sdk` and `supersonic-tx-cli`. CLI also switched `serde_json` to `serde_json.workspace = true`. Dependency resolution verified via `cargo metadata` and `cargo tree`. Full `cargo check` compile could not complete on this host due to Windows App Control policy (os error 4551) blocking build scripts.

---

## Environment setup (pre-task)

| Step | Result |
|------|--------|
| `cargo` not on PATH | Located at `%USERPROFILE%\.cargo\bin`; added to session PATH |
| `rustup` via winget | Installed Rust 1.97.1 stable MSVC |
| VS Build Tools 2022 | Installed with VCTools workload; `link.exe` available via VsDevCmd |
| App Control (WDAC) | Blocks execution of freshly built `build-script-build` binaries (error 4551) on both `H:` and `C:\temp` |

---

## TDD Step 1 — RED: failing compile probe

**Command:**
```powershell
cargo check -p supersonic-tx-sdk
```

**Exit code:** 101

**Pre-fix state (source inspection):**
- `crates/supersonic-tx-sdk/src/builder.rs` calls `bincode::serialize` at lines 118 and 161.
- `crates/supersonic-tx-sdk/Cargo.toml` had **no** `bincode` dependency.
- `crates/supersonic-tx-cli/Cargo.toml` used inline `bincode = "1.3"` (not workspace-pinned).

**Pre-fix `cargo tree -p supersonic-tx-sdk --depth 1` (direct deps — no bincode):**
```
supersonic-tx-sdk v0.1.0
├── rand v0.8.7
├── rand_distr v0.4.3
├── solana-client v1.18.26
├── solana-program v1.18.26
├── solana-sdk v1.18.26
├── supersonic-tx-core v0.1.0
├── thiserror v1.0.69
├── tokio v1.53.1
└── tracing v0.1.44
```

**Observed compile failure (blocked before SDK crate check):**
```
error: failed to run custom build command for `proc-macro2 v1.0.107`
Caused by:
  could not execute process `...\build-script-build` (never executed)
Caused by:
  Uma política de Controle de Aplicativo bloqueou este arquivo. (os error 4551)
```

**Expected bincode error (not reached due to WDAC):** Rust requires direct crate declarations for `bincode::serialize` usage; expected `E0433` / unresolved crate `bincode` once build scripts can run.

**Known remaining API error (Task 4, not fixed here):**
- `builder.rs:104` — `VersionedMessage::try_compile(...)` (invalid API for Solana 1.18)

Log artifact: `.superpowers/sdd/briefs/task-2-red.log`

---

## TDD Step 2 — Record failure output

Captured in RED section above. No `can't find crate \`bincode\`` reached compile stage; infrastructure blocker (4551) prevented SDK source compilation. Source + manifest gap confirms the dependency defect this task addresses.

---

## TDD Step 3 — Minimal dep fix

### Root `Cargo.toml`
Added under `[workspace.dependencies]`:
```toml
bincode = "1.3"
```

### `crates/supersonic-tx-sdk/Cargo.toml`
Added:
```toml
bincode.workspace = true
```

### `crates/supersonic-tx-cli/Cargo.toml`
Replaced `bincode = "1.3"` with:
```toml
bincode.workspace = true
serde_json.workspace = true
```

**Not changed:** `builder.rs`, `VersionedMessage::try_compile`, or any product logic.

---

## TDD Step 4 — GREEN: re-check SDK deps resolve

**Command:**
```powershell
cargo metadata --no-deps -q
cargo check -p supersonic-tx-sdk 2>&1 | Select-String -Pattern "bincode|try_compile|error"
```

**`cargo metadata --no-deps`:** OK (exit 0). SDK manifest now lists `bincode` with `req ^1.3`.

**Post-fix `cargo tree -p supersonic-tx-sdk --depth 1`:**
```
supersonic-tx-sdk v0.1.0
├── bincode v1.3.3          ← direct workspace dep (GREEN)
├── rand v0.8.7
...
```

**Post-fix `cargo tree -p supersonic-tx-cli --depth 1`:**
```
supersonic-tx-cli v0.1.0
├── bincode v1.3.3          ← workspace dep (GREEN)
├── serde_json v1.0.151     ← workspace dep (GREEN)
...
```

**Filtered `cargo check` output:** No `can't find crate \`bincode\`` or `bincode` errors. Remaining error is WDAC build-script block (4551), not a dependency resolution failure.

**Expected Task 4 error still present in source:** `VersionedMessage::try_compile` at `builder.rs:104` — not addressed in this task.

Log artifact: `.superpowers/sdd/briefs/task-2-green.log`

---

## TDD Step 5 — Commit

```bash
git add Cargo.toml crates/supersonic-tx-sdk/Cargo.toml crates/supersonic-tx-cli/Cargo.toml
git commit -m "build: add workspace bincode for SDK and CLI"
```

**Result:** `6b7e922 build: add workspace bincode for SDK and CLI` (3 files, +77 lines)

---

## Self-review

| Check | Pass |
|-------|------|
| Workspace pin `bincode = "1.3"` in root | ✅ |
| SDK uses `bincode.workspace = true` | ✅ |
| CLI uses `bincode.workspace = true` | ✅ |
| CLI uses `serde_json.workspace = true` | ✅ |
| No `VersionedMessage::try_compile` changes | ✅ |
| No product logic changes | ✅ |
| Commit message matches brief | ✅ |
| Only specified files committed | ✅ |
| `cargo metadata` resolves bincode for SDK/CLI | ✅ |
| Full `cargo check` compile green | ⚠️ Blocked by WDAC 4551 on this host |

---

## Concerns

1. **WDAC / App Control policy (os error 4551)** prevents running Cargo build scripts on this machine, so full compile verification (including confirming absence of bincode E0433 and presence of try_compile E0599) could not be completed. Dependency wiring is verified via `cargo metadata` and `cargo tree`.
2. **Large untracked project tree** — only the three Cargo.toml files were committed per brief; remaining crates/sources are still untracked on `feature/bar-c` (pre-existing repo state).

---

## Files modified

| File | Change |
|------|--------|
| `Cargo.toml` | Added `bincode = "1.3"` to `[workspace.dependencies]` |
| `crates/supersonic-tx-sdk/Cargo.toml` | Added `bincode.workspace = true` |
| `crates/supersonic-tx-cli/Cargo.toml` | `bincode.workspace = true`, `serde_json.workspace = true` |

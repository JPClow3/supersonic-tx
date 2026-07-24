# Task 3 Report — Real program ID plumbing

**Status:** DONE_WITH_CONCERNS
**Branch:** `feature/bar-c`
**Commit:** `67fed6c` — `feat: replace placeholder program id with generated key pubkey`
**Date:** 2026-07-23

---

## Summary

Generated a local Solana Ed25519 keypair at
`target/deploy/supersonic_tx-keypair.json` and synchronized its public key,
`GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`, across the Anchor program,
core crate, and both Anchor cluster configurations. Added the requested
`program_id()` helper and inline non-placeholder/parse consistency test.
Previously untracked workspace sources and manifests required for this task
were included in the commit. The keypair remains ignored and uncommitted.

---

## TDD Step 1 — RED test

Added to `crates/supersonic-tx-core/src/lib.rs`:

```rust
#[test]
fn program_id_is_not_placeholder() {
    assert_ne!(
        PROGRAM_ID_STR,
        "Super11111111111111111111111111111111111111"
    );
    let pk = Pubkey::from_str(PROGRAM_ID_STR).expect("PROGRAM_ID_STR must be valid base58");
    assert_eq!(pk, program_id());
}
```

Also added:

```rust
pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID_STR).expect("PROGRAM_ID_STR must be valid")
}
```

The initial attempt to run the RED command failed because `cargo` was not on
the session `PATH`. The explicit toolchain-path retry reached compilation but
was blocked before the test by Windows App Control / WDAC error 4551.

---

## TDD Step 2 — RED evidence

Command:

```powershell
cargo test -p supersonic-tx-core program_id_is_not_placeholder -- --nocapture
```

Initial result:

```text
cargo : The term 'cargo' is not recognized as the name of a cmdlet...
```

Retry using `%USERPROFILE%\.cargo\bin\cargo.exe`:

```text
error: failed to run custom build command for `proc-macro2 v1.0.107`
Caused by:
  could not execute process `...\proc-macro2-...\build-script-build` (never executed)
Caused by:
  Uma política de Controle de Aplicativo bloqueou este arquivo. (os error 4551)
```

Therefore the expected assertion failure could not be observed at runtime.

---

## TDD Step 3 — Generate and synchronize ID

Generated keypair JSON using a one-off RFC 8032 Ed25519 implementation because
`solana-keygen` was unavailable:

```text
target/deploy/supersonic_tx-keypair.json
```

Derived public key:

```text
GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9
```

Synchronized locations:

- `programs/supersonic-tx/src/lib.rs`: `declare_id!(...)`
- `crates/supersonic-tx-core/src/lib.rs`: `PROGRAM_ID_STR`
- `crates/supersonic-tx-core/src/lib.rs`: `program_id()`
- `Anchor.toml`: `[programs.localnet] supersonic_tx`
- `Anchor.toml`: `[programs.devnet] supersonic_tx`

---

## TDD Step 4 — GREEN evidence

The Cargo test could not complete because WDAC blocked the `proc-macro2`
build script. Static and keypair validation succeeded:

- Keypair JSON contains exactly 64 bytes.
- Base58 encoding of bytes 32–63 equals
  `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9`.
- `declare_id!`, `PROGRAM_ID_STR`, and both Anchor entries match exactly.
- `rg` found no placeholder in the actual ID declarations/configuration; the
  only remaining placeholder text in the core source is the negative test
  fixture.
- `target/deploy/supersonic_tx-keypair.json` is covered by `.gitignore`.

---

## TDD Step 5 — Commit

```powershell
git add Anchor.toml Cargo.lock crates/supersonic-tx-core crates/supersonic-tx-sdk crates/supersonic-tx-cli programs
git commit -m "feat: replace placeholder program id with generated key pubkey"
```

Result:

```text
67fed6c feat: replace placeholder program id with generated key pubkey
```

The commit includes the previously untracked workspace sources and manifests
needed for a self-contained task state. It does not include `.agents/`,
keypairs, handoff JSON, secrets, or the report itself.

---

## Self-review

| Check | Result |
|---|---|
| Real generated public key | Pass |
| Keypair parses as 64-byte Solana JSON | Pass |
| `declare_id!` synchronized | Pass |
| Core `PROGRAM_ID_STR` synchronized | Pass |
| Core `program_id()` parses and returns the constant | Pass by source/static validation |
| Anchor localnet/devnet synchronized | Pass |
| Placeholder absent from actual ID plumbing | Pass |
| Keypair not staged/committed | Pass |
| Required previously untracked sources staged | Pass |
| No whitespace errors in staged diff | Pass |
| Requested Cargo test green | Blocked by WDAC 4551 |

## Concerns

1. Runtime Cargo test evidence is unavailable on this host because Windows App
   Control blocks Rust build scripts with os error 4551. The keypair, base58,
   source synchronization, and ignore behavior were independently validated.
2. Pre-existing unrelated untracked files remain in the working tree:
   `.superpowers/`, `ARCHITECTURE.md`, `README.md`, and `docs/superpowers/`.

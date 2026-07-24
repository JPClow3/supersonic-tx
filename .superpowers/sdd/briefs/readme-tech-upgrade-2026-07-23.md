# README tech upgrade — 2026-07-23

Branch: `feature/bar-c`

## Goal

Rewrite `README.md` for systems engineers / Solana privacy builders: denser technical
depth, no marketing fluff, no invented APIs. Docs-only.

## Sources verified

- Spec `docs/superpowers/specs/2026-07-23-supersonic-tx-design.md`
- `ARCHITECTURE.md`, `docs/deploy.md`, `docs/smoke.md`
- Closeout `.superpowers/sdd/briefs/bar-c-closeout-2026-07-23.md`
- CLI flags: `crates/supersonic-tx-cli/src/main.rs` (clap `Commands`)
- Program: `programs/supersonic-tx/src/lib.rs`
- SDK: builder shrink order, `TrustedSystemAccount`, `simulate_and_send` confirm path
- Cooker: schema v1, overwrite refusal, fund blockhash refresh, sink provenance

## Corrections vs prior README

- Removed spurious `campaign --via-router` (flag exists only on `simulate` / `cast`)
- Documented confirm-send (`send_and_confirm_transaction`), campaign per-tx blockhash
  refresh, cooker fund blockhash refresh, and cooker sink `secret_key_path` provenance
- Kept Deployments localnet PASS / devnet blocked accurate

## Sections added / expanded

System model · threat model · bundle pipeline · program surface · account-cooker ·
campaign · build/test/deploy (dual-lock) · CLI reference · limits/ops · deployments

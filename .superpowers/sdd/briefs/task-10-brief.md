# Task 10 Brief

Extracted from plan. Use values verbatim.

## Global Constraints (binding)

- Workspace root: `H:\Code\Pessoais\SP - Solana\supersonic-tx\`
- CLI binary `supersonic-tx`; cook subcommand
- Handoff schema_version=1; relative paths
- Spec: docs/superpowers/specs/2026-07-23-supersonic-tx-design.md

## Controller notes

- Branch `feature/bar-c`; HEAD `1f1eb31`
- Tasks 1–9 complete
- Minors for final: Task 9 unused `_handoff_dir`, `/tmp` in test
- Parallel prod-bugfix may touch CLI — coordinate carefully
- Do not commit secrets

---
### Task 10: CLI `cook` subcommand

**Files:**
- Modify: `crates/supersonic-tx-cli/src/main.rs`
- Modify: `crates/supersonic-tx-cli/Cargo.toml` (`account-cooker` path dep)

**Interfaces:**
- Consumes: `Cooker`, `CookerConfig`
- Produces: CLI

```text
supersonic-tx cook \
  --sponsor-keypair <PATH> \
  --out-dir <DIR> \
  --rpc-url <URL> \
  --sinks <N> \
  --fee-payer-lamports <u64> \
  --sink-lamports <u64> \
  --cluster <name>
```

Writes `{out_dir}/keys/*` and `{out_dir}/handoff-<unix_ts>.json`. Funding requires RPC; add `--dry-run` that skips fund but still writes keypairs + handoff with configured lamport fields (document that dry-run is not cast-ready until funded).

- [ ] **Step 1: Write failing clap test**

```rust
#[test]
fn test_cli_parse_cook() {
    let args = vec![
        "supersonic-tx", "cook",
        "--sponsor-keypair", "/tmp/sponsor.json",
        "--out-dir", "/tmp/cook",
        "--rpc-url", "https://api.devnet.solana.com",
        "--sinks", "2",
    ];
    let cli = Cli::try_parse_from(args).expect("cook parse");
    match cli.command {
        Commands::Cook { sponsor_keypair, out_dir, sinks, .. } => {
            assert_eq!(sponsor_keypair, "/tmp/sponsor.json");
            assert_eq!(out_dir, "/tmp/cook");
            assert_eq!(sinks, 2);
        }
        _ => panic!("expected Cook"),
    }
}
```

- [ ] **Step 2: Run fail**

```bash
cargo test -p supersonic-tx-cli test_cli_parse_cook -- --nocapture
```

- [ ] **Step 3: Implement Cook variant + handler**

Wire `account-cooker` dependency:

```toml
account-cooker = { path = "../account-cooker" }
```

Implement command handler: generate â†’ write keys â†’ optional fund â†’ write handoff â†’ print path + warnings to stderr.

- [ ] **Step 4: Run pass**

```bash
cargo test -p supersonic-tx-cli test_cli_parse_cook -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add crates/supersonic-tx-cli crates/account-cooker Cargo.toml
git commit -m "feat: CLI cook subcommand writing handoff JSON"
```

---

# Milestone M3 â€” Program tests + deployability


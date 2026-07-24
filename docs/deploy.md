# Deploying `supersonic_tx`

## Security

**Never commit** `target/deploy/*-keypair.json`, sponsor keys, cooked account keys, or
ephemeral deployer wallets. Store deployment authority in a secret manager with restricted
access. This repository intentionally contains **no funded deployer** for public clusters.

---

## Program identity

Sync before any deploy:

| Location | Must match |
| --- | --- |
| `programs/supersonic-tx/src/lib.rs` `declare_id!` | `GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` |
| `crates/supersonic-tx-core` `PROGRAM_ID_STR` | same |
| `Anchor.toml` `[programs.*]` | same |
| `target/deploy/supersonic_tx-keypair.json` | `solana-keygen pubkey` → same |

---

## Build artifacts (dual-lock)

Do **not** rely on monolithic `anchor build` alone if full-workspace metadata fails on your
host (edition2024 / Cargo 1.79 under Anchor 0.30.1).

### SBF + IDL (recommended)

```bash
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target \
  backpackapp/build:v0.30.1 \
  bash .superpowers/sdd/briefs/bar-c-build-sbf-only.sh
```

Script swaps in `.superpowers/sdd/briefs/Cargo.lock.sbf.v3`, slim workspace members
(program + core only), runs `cargo build-sbf … -- --locked`, then `anchor idl build -p supersonic_tx`.

Outputs:

- `target/deploy/supersonic_tx.so` (~196640 bytes at bar-C closeout)
- `target/idl/supersonic_tx.json`

### Native verification

```bash
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target rust:latest \
  cargo test --workspace --locked
```

### CLI binary (for smoke)

```bash
docker run --rm -v "$PWD:/workspace" \
  -v supersonic-cargo-registry:/usr/local/cargo/registry \
  -v supersonic-cargo-git:/usr/local/cargo/git \
  -v supersonic-target:/workspace-target \
  -w /workspace -e CARGO_TARGET_DIR=/workspace-target rust:latest \
  cargo build --release -p supersonic-tx-cli --locked
```

Lock policy: [ARCHITECTURE.md](../ARCHITECTURE.md#build-and-test-layout).

---

## Localnet (validated path)

Used for bar-C smoke PASS (2026-07-23). Genesis hash changes on every validator `--reset`.

### 1. Start validator

```bash
docker run -d --name supersonic-localnet \
  -p 8899:8899 -p 8900:8900 \
  backpackapp/build:v0.30.1 \
  solana-test-validator --reset --quiet
```

### 2. Configure deployer (ephemeral keypair, not in repo)

```bash
solana config set --url http://127.0.0.1:8899 --keypair /path/to/deployer.json
solana airdrop 10
```

Localnet airdrop succeeds; public devnet faucet may rate-limit (HTTP 429).

### 3. Deploy program

```bash
solana program deploy target/deploy/supersonic_tx.so \
  --program-id target/deploy/supersonic_tx-keypair.json

solana program show GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9
```

### 4. Smoke

Follow [smoke.md](smoke.md). Record deploy + cast signatures in your operator log.

---

## Devnet

**Blocked at bar-C closeout** without a funded deployer wallet. Public faucet returned
HTTP 429; `solana program show GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9` reports
account not found.

When you have a funded keypair:

```bash
solana config set --url devnet --keypair /path/to/deployer.json
solana config get
solana balance   # must be > 0 — do not assume faucet works

# Option A: anchor deploy (if full anchor build is green in your environment)
anchor deploy --provider.cluster devnet

# Option B: solana program deploy (after dual-lock .so build)
solana program deploy target/deploy/supersonic_tx.so \
  --program-id target/deploy/supersonic_tx-keypair.json

solana program show GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9
```

Then run [smoke.md](smoke.md) with `--rpc-url https://api.devnet.solana.com --cluster devnet`.

Update README **Deployments** table with UTC timestamp, genesis hash, deploy signature, and
smoke signature. Mainnet is optional and explicit—not required for bar C.

---

## Post-deploy checklist

1. `cargo test --workspace --locked` (and router crate tests if touched)
2. SBF artifact from dual-lock script (or green `anchor build` in CI)
3. `solana program show` — executable, loader-owned
4. Smoke: `cook` → `cast --via-router --send` (see [smoke.md](smoke.md))
5. Record: cluster, genesis hash, program ID, deploy sig, smoke sig, commit SHA

Evidence briefs: `.superpowers/sdd/briefs/bar-c-closeout-2026-07-23.md`,
`.superpowers/sdd/briefs/bar-c-smoke-2026-07-23.md`.

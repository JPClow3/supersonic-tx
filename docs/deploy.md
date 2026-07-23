# Deploying `supersonic_tx`

Never commit `target/deploy/*-keypair.json`, sponsor keys, or cooked account keys.
Back up deployment authority and program keypairs in an access-controlled secret store.

1. Select and verify the wallet and cluster:
   `solana config set --url devnet --keypair <DEPLOYER.json>` then
   `solana config get` and `solana genesis-hash`.
2. Sync IDs: `declare_id!`, `supersonic_tx_core::PROGRAM_ID_STR`, `Anchor.toml` must match
   `solana-keygen pubkey target/deploy/supersonic_tx-keypair.json`.
3. `anchor build`
4. `solana airdrop 2` (devnet) to provider wallet if needed
5. `anchor deploy --provider.cluster devnet`
6. Verify executable loader ownership with `solana program show <PROGRAM_ID>`.
7. Smoke: `supersonic-tx cook ...` → `simulate` → `cast ... --via-router --send`.
8. Record: UTC time, cluster/genesis hash, program ID, loader owner, deploy signature,
   smoke signature, commit SHA, `cargo test --workspace --locked`, and `anchor build`.
9. Mainnet is optional and explicit — not required for v1 bar C.

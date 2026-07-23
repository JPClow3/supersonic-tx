# Deploying `supersonic_tx`

1. Sync IDs: `declare_id!`, `supersonic_tx_core::PROGRAM_ID_STR`, `Anchor.toml` must match
   `solana-keygen pubkey target/deploy/supersonic_tx-keypair.json`.
2. `anchor build`
3. `solana airdrop 2` (devnet) to provider wallet if needed
4. `anchor deploy --provider.cluster devnet`
5. Record program id under README Deployments
6. Smoke: `supersonic-tx cook ...` → `simulate` → `cast ... --send` on devnet
7. Mainnet is optional and explicit — not required for v1 bar C

# Deploying the router

No cluster deployment is implied by the checked-in program ID.

1. Confirm `declare_id!`, `supersonic_tx_core::PROGRAM_ID_STR`, and both entries in
   `Anchor.toml` equal `solana-keygen pubkey target/deploy/supersonic_tx-keypair.json`.
2. Run `cargo test --workspace --locked` and `anchor build`.
3. Select devnet and fund the provider wallet.
4. Run `anchor deploy --provider.cluster devnet`.
5. Verify `solana program show <PROGRAM_ID> --url devnet` reports an executable account.
6. Record the signature and deployment slot in this document before advertising it.
7. Run a funded `cook`, `simulate`, and opt-in `cast --via-router` smoke test.

Mainnet deployment is a separate operator decision. Never commit deploy keypairs.

## Recorded deployments

None.

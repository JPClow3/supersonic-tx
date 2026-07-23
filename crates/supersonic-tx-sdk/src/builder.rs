use crate::noise::{
    AnchorRouterNoise, ComputeBudgetNoise, DecoyGenerator, StatisticalTransferNoise, MemoNoise,
};
use rand::seq::SliceRandom;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;
use supersonic_tx_core::types::{BundleManifest, ObfuscationLevel, SupersonicError};
use supersonic_tx_core::MAX_TX_PAYLOAD_BYTES;

/// Builder for constructing fuzzy transaction bundles.
pub struct FuzzyBundleBuilder {
    payer: Pubkey,
    level: ObfuscationLevel,
    target_instructions: Vec<Instruction>,
    generators: Vec<Box<dyn DecoyGenerator>>,
}

impl FuzzyBundleBuilder {
    pub fn new(payer: Pubkey, level: ObfuscationLevel) -> Self {
        Self {
            payer,
            level,
            target_instructions: Vec::new(),
            generators: vec![
                Box::new(StatisticalTransferNoise::default_mainnet_destinations()),
                Box::new(ComputeBudgetNoise::default()),
                Box::new(AnchorRouterNoise::default()),
                Box::new(MemoNoise::default()),
            ],
        }
    }

    /// Add a real target instruction to be hidden inside the fuzzy bundle.
    pub fn add_target_instruction(mut self, ix: Instruction) -> Self {
        self.target_instructions.push(ix);
        self
    }

    /// Add multiple target instructions.
    pub fn add_target_instructions(mut self, ixs: impl IntoIterator<Item = Instruction>) -> Self {
        self.target_instructions.extend(ixs);
        self
    }

    /// Register a custom decoy generator strategy.
    pub fn with_decoy_generator(mut self, generator: impl DecoyGenerator + 'static) -> Self {
        self.generators.push(Box::new(generator));
        self
    }

    /// Build the bundle manifest with interleaved target and decoy instructions.
    pub fn build_manifest(&self) -> Result<BundleManifest, SupersonicError> {
        let mut manifest = BundleManifest::new(self.level);
        manifest.target_instructions = self.target_instructions.clone();

        // Generate decoys from all registered noise generators
        for gen in &self.generators {
            let decoys = gen.generate_decoys(&self.payer, self.level);
            manifest.decoy_instructions.extend(decoys);
        }

        // Shuffle execution order to obscure signal position within the bundle
        let total_count = manifest.len();
        let mut order: Vec<usize> = (0..total_count).collect();
        let mut rng = rand::thread_rng();
        order.shuffle(&mut rng);
        manifest.execution_order = order;

        Ok(manifest)
    }

    /// Assemble all instructions (decoys + targets) into an ordered vector.
    pub fn assemble_instructions(manifest: &BundleManifest) -> Vec<Instruction> {
        let mut all_ixs = Vec::with_capacity(manifest.len());
        all_ixs.extend(manifest.decoy_instructions.clone());
        all_ixs.extend(manifest.target_instructions.clone());

        // Reorder based on execution order
        let mut reordered = Vec::with_capacity(manifest.len());
        for &idx in &manifest.execution_order {
            if idx < all_ixs.len() {
                reordered.push(all_ixs[idx].clone());
            }
        }
        reordered
    }

    /// Build a VersionedTransaction with V0 message and check MTU payload size (1232 bytes max).
    pub fn build_versioned_transaction(
        &self,
        recent_blockhash: Hash,
        address_lookup_table_accounts: &[AddressLookupTableAccount],
    ) -> Result<VersionedTransaction, SupersonicError> {
        let mut manifest = self.build_manifest()?;

        loop {
            let instructions = Self::assemble_instructions(&manifest);

            let message = VersionedMessage::try_compile(
                &self.payer,
                &instructions,
                address_lookup_table_accounts,
                recent_blockhash,
            )
            .map_err(|e| SupersonicError::InvalidDecoyConfig(format!("VersionedMessage compilation failed: {e}")))?;

            let num_signatures = message.header().num_required_signatures as usize;
            let tx = VersionedTransaction {
                signatures: vec![Signature::default(); num_signatures],
                message,
            };

            let serialized_bytes = bincode::serialize(&tx)
                .map_err(|e| SupersonicError::SerializationError(e.to_string()))?;

            let size = serialized_bytes.len();
            if size <= MAX_TX_PAYLOAD_BYTES {
                return Ok(tx);
            }

            // Exceeded size. Try dropping a decoy instruction to shrink the transaction.
            if manifest.decoy_instructions.is_empty() {
                return Err(SupersonicError::TransactionSizeExceeded(size));
            }

            manifest.decoy_instructions.pop();

            // Re-shuffle execution order
            let total_count = manifest.target_instructions.len() + manifest.decoy_instructions.len();
            let mut order: Vec<usize> = (0..total_count).collect();
            let mut rng = rand::thread_rng();
            order.shuffle(&mut rng);
            manifest.execution_order = order;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_bundle_builder_versioned_transaction_compilation() {
        let payer = Pubkey::new_unique();
        let target = Pubkey::new_unique();
        let ix = solana_sdk::system_instruction::transfer(&payer, &target, 10_000);

        let builder = FuzzyBundleBuilder::new(payer, ObfuscationLevel::Standard)
            .add_target_instruction(ix);

        let blockhash = Hash::new_unique();
        let res = builder.build_versioned_transaction(blockhash, &[]);
        assert!(res.is_ok());

        let tx = res.unwrap();
        let serialized = bincode::serialize(&tx).unwrap();
        assert!(serialized.len() <= supersonic_tx_core::MAX_TX_PAYLOAD_BYTES);
    }

    #[test]
    fn test_transaction_mtu_size_verification() {
        let payer = Pubkey::new_unique();
        let mut builder = FuzzyBundleBuilder::new(payer, ObfuscationLevel::Paranoid);

        // Force payload size beyond 1232 bytes MTU limit by adding many instructions with large payloads
        for _ in 0..40 {
            let dummy_program = Pubkey::new_unique();
            let large_data = vec![0u8; 100];
            let ix = Instruction {
                program_id: dummy_program,
                accounts: vec![
                    solana_sdk::instruction::AccountMeta::new(Pubkey::new_unique(), false),
                    solana_sdk::instruction::AccountMeta::new_readonly(Pubkey::new_unique(), false),
                ],
                data: large_data,
            };
            builder = builder.add_target_instruction(ix);
        }

        let blockhash = Hash::new_unique();
        let res = builder.build_versioned_transaction(blockhash, &[]);
        assert!(res.is_err());

        match res {
            Err(SupersonicError::TransactionSizeExceeded(size)) => {
                assert!(size > 1232, "Exceeded size should be > 1232, got {}", size);
            }
            other => panic!("Expected TransactionSizeExceeded, got {:?}", other),
        }
    }
}

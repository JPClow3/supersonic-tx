use crate::noise::{
    AnchorRouterNoise, ComputeBudgetNoise, DecoyGenerator, MemoNoise, StatisticalTransferNoise,
};
use rand::seq::SliceRandom;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::compute_budget;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::{v0, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;
use std::str::FromStr;
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
                Box::new(ComputeBudgetNoise::default()),
                Box::new(AnchorRouterNoise::default()),
                Box::new(MemoNoise::default()),
            ],
        }
    }

    /// Add fail-soft SOL transfer sinks for statistical decoys.
    pub fn with_sinks(mut self, sinks: Vec<Pubkey>) -> Self {
        if !sinks.is_empty() {
            if let Ok(noise) = StatisticalTransferNoise::from_cooked_sinks(sinks) {
                self.generators.push(Box::new(noise));
            }
        }
        self
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

    /// Compile a V0 message and estimate its serialized transaction size.
    pub fn compile_v0_message(
        payer: &Pubkey,
        instructions: &[Instruction],
        address_lookup_table_accounts: &[AddressLookupTableAccount],
        recent_blockhash: Hash,
    ) -> Result<VersionedMessage, SupersonicError> {
        let message = v0::Message::try_compile(
            payer,
            instructions,
            address_lookup_table_accounts,
            recent_blockhash,
        )
        .map_err(|e| {
            SupersonicError::InvalidDecoyConfig(format!("v0::Message::try_compile failed: {e}"))
        })?;
        Ok(VersionedMessage::V0(message))
    }

    /// Estimate serialized size using placeholder signatures. This does not sign a transaction.
    pub fn estimate_tx_size(message: &VersionedMessage) -> Result<usize, SupersonicError> {
        let num_signatures = message.header().num_required_signatures as usize;
        let tx = VersionedTransaction {
            signatures: vec![Signature::default(); num_signatures],
            message: message.clone(),
        };
        let serialized_bytes = bincode::serialize(&tx)
            .map_err(|e| SupersonicError::SerializationError(e.to_string()))?;
        Ok(serialized_bytes.len())
    }

    /// Build an unsigned V0 message, shrinking decoys until its estimated size fits the MTU.
    pub fn build_versioned_message(
        &self,
        recent_blockhash: Hash,
        address_lookup_table_accounts: &[AddressLookupTableAccount],
    ) -> Result<VersionedMessage, SupersonicError> {
        let mut manifest = self.build_manifest()?;

        loop {
            let instructions = Self::assemble_instructions(&manifest);
            let message = Self::compile_v0_message(
                &self.payer,
                &instructions,
                address_lookup_table_accounts,
                recent_blockhash,
            )?;
            let size = Self::estimate_tx_size(&message)?;
            if size <= MAX_TX_PAYLOAD_BYTES {
                return Ok(message);
            }

            if !Self::shrink_decoys(&mut manifest) {
                return Err(SupersonicError::TransactionSizeExceeded(size));
            }
        }
    }

    /// Deprecated compatibility wrapper returning placeholder signatures for estimation only.
    ///
    /// The returned transaction is not signed and must not be submitted. Use
    /// [`Self::build_versioned_message`] and the signing API from Task 12 instead.
    #[deprecated(
        note = "returns placeholder signatures for estimation only; use build_versioned_message"
    )]
    pub fn build_versioned_transaction(
        &self,
        recent_blockhash: Hash,
        address_lookup_table_accounts: &[AddressLookupTableAccount],
    ) -> Result<VersionedTransaction, SupersonicError> {
        let message =
            self.build_versioned_message(recent_blockhash, address_lookup_table_accounts)?;
        let num_signatures = message.header().num_required_signatures as usize;
        Ok(VersionedTransaction {
            signatures: vec![Signature::default(); num_signatures],
            message,
        })
    }

    pub(crate) fn shrink_decoys(manifest: &mut BundleManifest) -> bool {
        const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
        let router_program_id = supersonic_tx_core::program_id();
        let memo_program_id = MEMO_PROGRAM_ID
            .parse::<Pubkey>()
            .expect("valid memo program id");
        let compute_budget_id = compute_budget::id();

        let position = manifest
            .decoy_instructions
            .iter()
            .position(|ix| ix.program_id == solana_sdk::system_program::id())
            .or_else(|| {
                manifest
                    .decoy_instructions
                    .iter()
                    .position(|ix| ix.program_id == memo_program_id)
            })
            .or_else(|| {
                let router_count = manifest
                    .decoy_instructions
                    .iter()
                    .filter(|ix| ix.program_id == router_program_id)
                    .count();
                (router_count > 1).then(|| {
                    manifest
                        .decoy_instructions
                        .iter()
                        .position(|ix| ix.program_id == router_program_id)
                        .expect("router count proves a router decoy exists")
                })
            })
            .or_else(|| {
                let last_compute_budget = manifest
                    .decoy_instructions
                    .iter()
                    .rposition(|ix| ix.program_id == compute_budget_id);
                manifest
                    .decoy_instructions
                    .iter()
                    .enumerate()
                    .find(|(index, ix)| {
                        ix.program_id != compute_budget_id || Some(*index) != last_compute_budget
                    })
                    .map(|(index, _)| index)
            });

        let Some(position) = position else {
            return false;
        };
        manifest.decoy_instructions.remove(position);
        let mut order: Vec<usize> = (0..manifest.len()).collect();
        order.shuffle(&mut rand::thread_rng());
        manifest.execution_order = order;
        true
    }

    #[cfg(test)]
    pub fn shrink_decoys_for_test(manifest: &mut BundleManifest) -> bool {
        Self::shrink_decoys(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_v0_message_without_versioned_message_try_compile() {
        let payer = Pubkey::new_unique();
        let target = Pubkey::new_unique();
        let ix = solana_sdk::system_instruction::transfer(&payer, &target, 10_000);

        let builder =
            FuzzyBundleBuilder::new(payer, ObfuscationLevel::Light).add_target_instruction(ix);
        let msg = builder
            .build_versioned_message(Hash::new_unique(), &[])
            .expect("v0 compile");
        match msg {
            solana_sdk::message::VersionedMessage::V0(_) => {}
            solana_sdk::message::VersionedMessage::Legacy(_) => panic!("expected V0"),
        }
        let size = FuzzyBundleBuilder::estimate_tx_size(&msg).unwrap();
        assert!(size <= MAX_TX_PAYLOAD_BYTES);
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
        let res = builder.build_versioned_message(blockhash, &[]);
        assert!(res.is_err());

        match res {
            Err(SupersonicError::TransactionSizeExceeded(size)) => {
                assert!(size > 1232, "Exceeded size should be > 1232, got {}", size);
            }
            other => panic!("Expected TransactionSizeExceeded, got {:?}", other),
        }
    }

    #[test]
    fn shrink_drops_statistical_before_memo() {
        let mut manifest = BundleManifest::new(ObfuscationLevel::Standard);
        let payer = Pubkey::new_unique();
        let sink = Pubkey::new_unique();
        manifest.decoy_instructions = vec![
            solana_sdk::system_instruction::transfer(&payer, &sink, 1000),
            Instruction {
                program_id: Pubkey::from_str(
                    "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr",
                )
                .unwrap(),
                accounts: vec![],
                data: b"x".to_vec(),
            },
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(200_000),
        ];
        assert!(FuzzyBundleBuilder::shrink_decoys_for_test(&mut manifest));
        assert!(
            manifest
                .decoy_instructions
                .iter()
                .all(|ix| ix.program_id != solana_sdk::system_program::id()
                    || ix.data.first() != Some(&2))
        );
        assert_eq!(manifest.decoy_instructions.len(), 2);
    }
}

use crate::noise::{
    AnchorRouterNoise, ComputeBudgetNoise, DecoyGenerator, DecoySink, MemoNoise,
    SinkValidationMode, StatisticalTransferNoise,
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
use supersonic_tx_core::types::{BundleManifest, ObfuscationLevel, SupersonicError};
use supersonic_tx_core::MAX_TX_PAYLOAD_BYTES;

#[derive(Debug, Clone)]
pub struct BuiltBundle {
    pub manifest: BundleManifest,
    pub message: VersionedMessage,
    pub serialized_size: usize,
}

/// Builder for constructing fuzzy transaction bundles.
pub struct FuzzyBundleBuilder {
    payer: Pubkey,
    level: ObfuscationLevel,
    target_instructions: Vec<Instruction>,
    generators: Vec<Box<dyn DecoyGenerator>>,
    transfer_noise_disabled: bool,
    transfer_sink_count: usize,
    sink_validation_mode: SinkValidationMode,
}

impl FuzzyBundleBuilder {
    pub fn new(payer: Pubkey, level: ObfuscationLevel) -> Self {
        Self {
            payer,
            level,
            target_instructions: Vec::new(),
            generators: vec![
                Box::new(ComputeBudgetNoise::default()),
                Box::new(MemoNoise::default()),
            ],
            transfer_noise_disabled: false,
            transfer_sink_count: 0,
            sink_validation_mode: SinkValidationMode::DenyListOnly,
        }
    }

    /// Add provenance-gated fail-soft SOL transfer sinks.
    pub fn with_sinks(mut self, sinks: Vec<DecoySink>) -> Result<Self, SupersonicError> {
        if sinks.is_empty() {
            return Err(SupersonicError::InvalidDecoyConfig(
                "transfer decoys require at least one validated sink".to_string(),
            ));
        }
        self.transfer_sink_count = sinks.len();
        self.generators
            .push(Box::new(StatisticalTransferNoise::from_sinks(sinks)));
        Ok(self)
    }

    /// Explicitly select a profile without statistical transfer noise.
    ///
    /// This is intentionally opt-in because every documented security level
    /// otherwise promises transfer decoys.
    pub fn without_transfer_noise(mut self) -> Self {
        self.transfer_noise_disabled = true;
        self
    }

    /// Opt into router noops only after the caller has verified deployment.
    pub fn with_router_noise(mut self, program_id: Pubkey) -> Self {
        self.generators
            .push(Box::new(AnchorRouterNoise::new(program_id)));
        self
    }

    /// Select sink validation policy. On-chain mode requires a checker that
    /// is not yet available in this SDK and therefore fails during build.
    pub fn with_sink_validation_mode(mut self, mode: SinkValidationMode) -> Self {
        self.sink_validation_mode = mode;
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

    /// Build the bundle manifest with interleaved target and decoy instructions.
    pub fn build_manifest(&self) -> Result<BundleManifest, SupersonicError> {
        match self.sink_validation_mode {
            SinkValidationMode::DenyListOnly => {}
            SinkValidationMode::RequireOnChainNonExecutable => {
                return Err(SupersonicError::InvalidDecoyConfig(
                    "on-chain non-executable sink validation requires an RPC checker".to_string(),
                ));
            }
        }
        if !self.transfer_noise_disabled && self.transfer_sink_count == 0 {
            return Err(SupersonicError::InvalidDecoyConfig(
                "this level requires validated transfer sinks; call with_sinks or explicitly select without_transfer_noise".to_string(),
            ));
        }
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

    /// Build one final manifest and compile that exact manifest into a V0 message.
    pub fn build_bundle(
        &self,
        recent_blockhash: Hash,
        address_lookup_table_accounts: &[AddressLookupTableAccount],
    ) -> Result<BuiltBundle, SupersonicError> {
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
                return Ok(BuiltBundle {
                    manifest,
                    message,
                    serialized_size: size,
                });
            }

            if !Self::shrink_decoys(&mut manifest) {
                return Err(SupersonicError::TransactionSizeExceeded(size));
            }
        }
    }

    /// Build an unsigned V0 message, shrinking decoys until its estimated size fits the MTU.
    pub fn build_versioned_message(
        &self,
        recent_blockhash: Hash,
        address_lookup_table_accounts: &[AddressLookupTableAccount],
    ) -> Result<VersionedMessage, SupersonicError> {
        Ok(self
            .build_bundle(recent_blockhash, address_lookup_table_accounts)?
            .message)
    }

    pub(crate) fn shrink_decoys(manifest: &mut BundleManifest) -> bool {
        const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
        let router_program_id = supersonic_tx_core::program_id();
        let memo_program_id = MEMO_PROGRAM_ID.parse::<Pubkey>().ok();
        let compute_budget_id = compute_budget::id();

        let position = manifest
            .decoy_instructions
            .iter()
            .position(|ix| ix.program_id == solana_sdk::system_program::id())
            .or_else(|| {
                manifest
                    .decoy_instructions
                    .iter()
                    .position(|ix| Some(ix.program_id) == memo_program_id)
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
                // Price-only padding is lower priority than the CU limit.
                manifest.decoy_instructions.iter().position(|ix| {
                    ix.program_id == compute_budget_id && ix.data.first() == Some(&3)
                })
            })
            .or_else(|| {
                let protected_limit = manifest.decoy_instructions.iter().position(|ix| {
                    ix.program_id == compute_budget_id && ix.data.first() == Some(&2)
                });
                manifest
                    .decoy_instructions
                    .iter()
                    .enumerate()
                    .find(|(index, ix)| {
                        ix.program_id != compute_budget_id || Some(*index) != protected_limit
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
    use std::str::FromStr;

    #[test]
    fn compiles_v0_message_without_versioned_message_try_compile() {
        let payer = Pubkey::new_unique();
        let target = Pubkey::new_unique();
        let ix = solana_sdk::system_instruction::transfer(&payer, &target, 10_000);

        let builder = FuzzyBundleBuilder::new(payer, ObfuscationLevel::Light)
            .without_transfer_noise()
            .add_target_instruction(ix);
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
        let mut builder =
            FuzzyBundleBuilder::new(payer, ObfuscationLevel::Paranoid).without_transfer_noise();

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
                program_id: Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
                    .unwrap(),
                accounts: vec![],
                data: b"x".to_vec(),
            },
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(200_000),
        ];
        assert!(FuzzyBundleBuilder::shrink_decoys_for_test(&mut manifest));
        assert!(manifest
            .decoy_instructions
            .iter()
            .all(|ix| ix.program_id != solana_sdk::system_program::id()
                || ix.data.first() != Some(&2)));
        assert_eq!(manifest.decoy_instructions.len(), 2);
    }

    #[test]
    fn default_builder_requires_explicit_transfer_policy() {
        let builder = FuzzyBundleBuilder::new(Pubkey::new_unique(), ObfuscationLevel::Standard);
        let error = builder.build_manifest().unwrap_err();
        assert!(error
            .to_string()
            .contains("requires validated transfer sinks"));
    }

    #[test]
    fn shrink_retains_compute_unit_limit_before_price() {
        let mut manifest = BundleManifest::new(ObfuscationLevel::Standard);
        manifest.decoy_instructions = vec![
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(10),
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(200_000),
        ];

        assert!(FuzzyBundleBuilder::shrink_decoys_for_test(&mut manifest));
        assert_eq!(manifest.decoy_instructions.len(), 1);
        assert_eq!(manifest.decoy_instructions[0].data.first(), Some(&2));
        assert!(!FuzzyBundleBuilder::shrink_decoys_for_test(&mut manifest));
    }
}

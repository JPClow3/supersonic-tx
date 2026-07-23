use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use supersonic_tx_core::{ObfuscationLevel, SupersonicError};

use crate::{DecoySink, FuzzyBundleBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedTxKind {
    DecoyOnly,
    RealIntent,
    PostNoise,
}

#[derive(Debug, Clone)]
pub struct PlannedTx {
    pub kind: PlannedTxKind,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct CampaignPlan {
    pub txs: Vec<PlannedTx>,
}

pub struct CampaignPlanner {
    payer: Pubkey,
    level: ObfuscationLevel,
    sinks: Vec<DecoySink>,
    isolate_intent: bool,
    decoy_tx_count: usize,
}

impl CampaignPlanner {
    pub fn new(payer: Pubkey, level: ObfuscationLevel) -> Self {
        Self {
            payer,
            level,
            sinks: Vec::new(),
            isolate_intent: true,
            decoy_tx_count: 2,
        }
    }

    pub fn with_sinks(mut self, sinks: Vec<DecoySink>) -> Self {
        self.sinks = sinks;
        self
    }

    pub fn isolate_intent(mut self, isolate: bool) -> Self {
        self.isolate_intent = isolate;
        self
    }

    pub fn decoy_tx_count(mut self, count: usize) -> Self {
        self.decoy_tx_count = count;
        self
    }

    pub fn plan(
        self,
        target_instructions: Vec<Instruction>,
    ) -> Result<CampaignPlan, SupersonicError> {
        if target_instructions.is_empty() {
            return Err(SupersonicError::InvalidCampaign(
                "real-intent transaction requires at least one target instruction".to_string(),
            ));
        }
        if self.decoy_tx_count > 0 && self.sinks.is_empty() {
            return Err(SupersonicError::InvalidCampaign(
                "decoy-only transactions require validated sinks".to_string(),
            ));
        }

        let mut txs = Vec::with_capacity(self.decoy_tx_count + 1);
        for _ in 0..self.decoy_tx_count {
            let manifest = FuzzyBundleBuilder::new(self.payer, self.level)
                .with_sinks(self.sinks.clone())?
                .build_manifest()?;
            txs.push(PlannedTx {
                kind: PlannedTxKind::DecoyOnly,
                instructions: FuzzyBundleBuilder::assemble_instructions(&manifest),
            });
        }

        let real_builder = FuzzyBundleBuilder::new(self.payer, self.level)
            .add_target_instructions(target_instructions);
        let real_manifest = if self.isolate_intent {
            real_builder.without_transfer_noise().build_manifest()?
        } else {
            real_builder.with_sinks(self.sinks)?.build_manifest()?
        };
        txs.push(PlannedTx {
            kind: PlannedTxKind::RealIntent,
            instructions: FuzzyBundleBuilder::assemble_instructions(&real_manifest),
        });

        Ok(CampaignPlan { txs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::system_instruction;

    #[test]
    fn isolated_plan_keeps_target_out_of_decoy_transactions() {
        let payer = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let target = system_instruction::transfer(&payer, &destination, 42);
        let sink_account = solana_sdk::account::Account {
            lamports: 1,
            data: Vec::new(),
            owner: solana_sdk::system_program::ID,
            executable: false,
            rent_epoch: 0,
        };
        let sink = DecoySink::from_rpc_account(Pubkey::new_unique(), &sink_account).unwrap();

        let plan = CampaignPlanner::new(payer, ObfuscationLevel::Standard)
            .with_sinks(vec![sink])
            .decoy_tx_count(2)
            .plan(vec![target.clone()])
            .unwrap();
        assert_eq!(
            plan.txs
                .iter()
                .filter(|transaction| transaction.kind == PlannedTxKind::DecoyOnly)
                .count(),
            2
        );
        for transaction in plan
            .txs
            .iter()
            .filter(|transaction| transaction.kind == PlannedTxKind::DecoyOnly)
        {
            assert!(!transaction.instructions.contains(&target));
        }
        let real = plan
            .txs
            .iter()
            .find(|transaction| transaction.kind == PlannedTxKind::RealIntent)
            .unwrap();
        assert!(real.instructions.contains(&target));
        assert!(real.instructions.iter().all(|instruction| {
            instruction == &target
                || instruction.program_id == solana_sdk::compute_budget::id()
                || instruction.program_id.to_string()
                    == "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"
        }));
    }
}

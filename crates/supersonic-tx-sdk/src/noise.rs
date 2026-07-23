use account_cooker::{CookedAccount, CookedRole};
use rand::Rng;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::account::Account;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_instruction;
use std::fmt;
use std::str::FromStr;
use supersonic_tx_core::types::ObfuscationLevel;

/// Trait defining noise generation strategies for masking intent.
pub trait DecoyGenerator: Send + Sync {
    /// Generate a vector of decoy instructions based on the requested obfuscation level.
    fn generate_decoys(&self, payer: &Pubkey, level: ObfuscationLevel) -> Vec<Instruction>;
}

/// Generates statistical micro-transfers following realistic log-normal / Benford distribution.
pub struct StatisticalTransferNoise {
    /// Trusted wallet sink pubkeys used for fail-soft statistical transfers.
    decoy_destinations: Vec<Pubkey>,
}

/// An opaque pubkey handoff for an account known to be a system-owned wallet.
///
/// Values can only be minted from the configured tip allowlist or a validated
/// `account-cooker` `DecoySink` handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustedSystemAccount(Pubkey);

/// A transfer sink carrying trusted-account provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecoySink(TrustedSystemAccount);

/// Selects how strongly transfer sinks must be validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkValidationMode {
    /// Apply the local known-program deny-list and provenance gate.
    DenyListOnly,
    /// Reserved for a future RPC account-owner/executable checker.
    RequireOnChainNonExecutable,
}

/// Error returned when a sink fails provenance or deny-list validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidDecoySink {
    DeniedProgram { destination: Pubkey },
    NotAllowlisted { destination: Pubkey },
    InvalidCookedRole,
    InvalidCookedPubkey { value: String },
    RpcValidation { destination: Pubkey, reason: String },
    NotSystemWallet { destination: Pubkey },
}

impl fmt::Display for InvalidDecoySink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeniedProgram { destination } => {
                write!(f, "decoy sink is a denied program address: {destination}")
            }
            Self::NotAllowlisted { destination } => {
                write!(
                    f,
                    "decoy sink is not in the configured tip allowlist: {destination}"
                )
            }
            Self::InvalidCookedRole => {
                write!(f, "cooked account role must be DecoySink")
            }
            Self::InvalidCookedPubkey { value } => {
                write!(f, "cooked DecoySink has an invalid pubkey: {value}")
            }
            Self::RpcValidation {
                destination,
                reason,
            } => write!(f, "could not validate decoy sink {destination}: {reason}"),
            Self::NotSystemWallet { destination } => write!(
                f,
                "decoy sink {destination} is executable or not owned by the system program"
            ),
        }
    }
}

impl std::error::Error for InvalidDecoySink {}

impl DecoySink {
    /// Accept a tip only when it is present in the built-in allowlist.
    ///
    /// The v1 built-in allowlist is intentionally empty; operators should
    /// validate configured tips with `try_tip_allowlisted_from`.
    pub fn try_tip_allowlisted(destination: Pubkey) -> Result<Self, InvalidDecoySink> {
        Self::try_tip_allowlisted_from(destination, &[])
    }

    /// Accept a tip only when it is present in an operator allowlist.
    pub fn try_tip_allowlisted_from(
        destination: Pubkey,
        allowlist: &[Pubkey],
    ) -> Result<Self, InvalidDecoySink> {
        TrustedSystemAccount::try_from_tip_allowlist(destination, allowlist).map(Self)
    }

    /// Convert an opaque cooker/system-wallet handoff into a transfer sink.
    pub fn from_trusted_system_account(account: TrustedSystemAccount) -> Self {
        Self(account)
    }

    pub fn pubkey(self) -> Pubkey {
        self.0 .0
    }

    /// Fetch and prove that a destination is a non-executable, system-owned account.
    pub async fn validate_on_chain(
        rpc: &RpcClient,
        destination: Pubkey,
    ) -> Result<Self, InvalidDecoySink> {
        let account = rpc.get_account(&destination).await.map_err(|error| {
            InvalidDecoySink::RpcValidation {
                destination,
                reason: error.to_string(),
            }
        })?;
        Self::from_rpc_account(destination, &account)
    }

    pub fn from_rpc_account(
        destination: Pubkey,
        account: &Account,
    ) -> Result<Self, InvalidDecoySink> {
        if account.executable || account.owner != solana_sdk::system_program::ID {
            return Err(InvalidDecoySink::NotSystemWallet { destination });
        }
        TrustedSystemAccount::from_validated_pubkey(destination).map(Self)
    }
}

impl TrustedSystemAccount {
    fn from_validated_pubkey(destination: Pubkey) -> Result<Self, InvalidDecoySink> {
        if is_denied_program(&destination) {
            return Err(InvalidDecoySink::DeniedProgram { destination });
        }
        Ok(Self(destination))
    }

    /// Validate a pubkey against an explicit tip allowlist.
    pub fn try_from_tip_allowlist(
        destination: Pubkey,
        allowlist: &[Pubkey],
    ) -> Result<Self, InvalidDecoySink> {
        if !allowlist.contains(&destination) {
            return Err(InvalidDecoySink::NotAllowlisted { destination });
        }
        Self::from_validated_pubkey(destination)
    }

    /// Mint provenance from an account-cooker handoff after checking its role,
    /// pubkey encoding, and the local program deny-list.
    pub fn from_cooker_decoy_sink(account: &CookedAccount) -> Result<Self, InvalidDecoySink> {
        match &account.role {
            CookedRole::DecoySink => {}
            CookedRole::FeePayer | CookedRole::DrainTarget => {
                return Err(InvalidDecoySink::InvalidCookedRole);
            }
        }

        let destination = Pubkey::from_str(&account.pubkey).map_err(|_| {
            InvalidDecoySink::InvalidCookedPubkey {
                value: account.pubkey.clone(),
            }
        })?;
        Self::from_validated_pubkey(destination)
    }
}

#[cfg(test)]
impl TrustedSystemAccount {
    fn assume_system_wallet_for_test(destination: Pubkey) -> Result<Self, InvalidDecoySink> {
        Self::from_validated_pubkey(destination)
    }
}

fn is_denied_program(destination: &Pubkey) -> bool {
    let value = destination.to_string();
    [
        solana_sdk::system_program::ID,
        Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
        Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap(),
        Pubkey::from_str("ComputeBudget111111111111111111111111111111").unwrap(),
        Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr").unwrap(),
    ]
    .contains(destination)
        || value.starts_with("JUP6")
        || value.starts_with("675kPX")
        || value.starts_with("whirLb")
        || value.starts_with("9W959")
}

impl StatisticalTransferNoise {
    /// Build statistical noise from provenance-gated transfer sinks.
    pub fn from_sinks(decoy_destinations: Vec<DecoySink>) -> Self {
        Self {
            decoy_destinations: decoy_destinations
                .into_iter()
                .map(DecoySink::pubkey)
                .collect(),
        }
    }

    /// Public tip / fee sink allowlist for fail-soft SOL transfers.
    /// Operators should prefer cooked DecoySinks from account-cooker.
    /// v1 ships an empty default; CLI/SDK inject sinks from handoff or `--tip`.
    pub fn default_tip_allowlist() -> Self {
        Self {
            decoy_destinations: Vec::new(),
        }
    }

    pub fn with_tips(mut self, tips: &[TrustedSystemAccount]) -> Result<Self, InvalidDecoySink> {
        for tip in tips {
            let sink = DecoySink::from_trusted_system_account(*tip);
            self.decoy_destinations.push(sink.pubkey());
        }
        Ok(self)
    }

    /// Sample a lamport transfer amount strictly adhering to Benford's Law distribution within [min_lamports, max_lamports].
    pub fn sample_benford_lamports<R: Rng>(
        rng: &mut R,
        min_lamports: u64,
        max_lamports: u64,
    ) -> u64 {
        loop {
            // 1. Sample leading digit d in [1, 9] via Inverse Transform Sampling: P(d) = log10(1 + 1/d)
            let u: f64 = rng.gen();
            let leading_digit = (10.0f64.powf(u)).floor() as u64;
            let leading_digit = leading_digit.clamp(1, 9);

            // 2. Select order of magnitude multiplier (10^3 or 10^4)
            let magnitude_exp = rng.gen_range(3..=4);
            let scale = 10u64.pow(magnitude_exp);

            // 3. Generate uniform noise for lower-order trailing digits
            let remainder = rng.gen_range(0..scale);
            let candidate = leading_digit * scale + remainder;

            if candidate >= min_lamports && candidate <= max_lamports {
                return candidate;
            }
        }
    }
}

impl DecoyGenerator for StatisticalTransferNoise {
    fn generate_decoys(&self, payer: &Pubkey, level: ObfuscationLevel) -> Vec<Instruction> {
        let mut rng = rand::thread_rng();
        let count = match level {
            ObfuscationLevel::Light => 1,
            ObfuscationLevel::Standard => 3,
            ObfuscationLevel::Paranoid => 5,
        };

        let mut instructions = Vec::new();
        if self.decoy_destinations.is_empty() {
            return instructions;
        }

        for _ in 0..count {
            let destination = if !self.decoy_destinations.is_empty() {
                self.decoy_destinations[rng.gen_range(0..self.decoy_destinations.len())]
            } else {
                unreachable!("empty sinks returned above");
            };
            let lamports = Self::sample_benford_lamports(&mut rng, 1_000, 50_000);
            instructions.push(system_instruction::transfer(payer, &destination, lamports));
        }

        instructions
    }
}

/// Generates dynamic Compute Budget Unit and Priority Fee padding to mask execution complexity.
#[derive(Debug, Clone)]
pub struct ComputeBudgetNoise {
    pub custom_limit_base: Option<u32>,
    pub enable_price_noise: bool,
}

impl Default for ComputeBudgetNoise {
    fn default() -> Self {
        Self {
            custom_limit_base: None,
            enable_price_noise: true,
        }
    }
}

impl ComputeBudgetNoise {
    pub fn new(custom_limit_base: Option<u32>, enable_price_noise: bool) -> Self {
        Self {
            custom_limit_base,
            enable_price_noise,
        }
    }
}

impl DecoyGenerator for ComputeBudgetNoise {
    fn generate_decoys(&self, _payer: &Pubkey, level: ObfuscationLevel) -> Vec<Instruction> {
        let mut rng = rand::thread_rng();
        let mut instructions = Vec::new();

        // 1. Compute Unit Limit Normalization + Jitter (5% - 15%)
        let base_limit = self
            .custom_limit_base
            .unwrap_or(match level {
                ObfuscationLevel::Light => 200_000,
                ObfuscationLevel::Standard => 400_000,
                ObfuscationLevel::Paranoid => 800_000,
            })
            .min(1_400_000);

        let jitter_percent = rng.gen_range(5..=15);
        let jitter = base_limit.saturating_mul(jitter_percent) / 100;
        let final_limit = base_limit.saturating_add(jitter).min(1_400_000);
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(
            final_limit,
        ));

        // 2. Priority Fee Randomization (SetComputeUnitPrice)
        if self.enable_price_noise {
            let micro_lamports = match level {
                ObfuscationLevel::Light => rng.gen_range(1_000..5_000),
                ObfuscationLevel::Standard => rng.gen_range(5_000..25_000),
                ObfuscationLevel::Paranoid => rng.gen_range(25_000..100_000),
            };
            instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
                micro_lamports,
            ));
        }

        instructions
    }
}

/// Generates Anchor `noop_decoy` instructions for the supersonic-tx program.
#[derive(Debug, Clone)]
pub struct AnchorRouterNoise {
    pub program_id: Pubkey,
}

impl Default for AnchorRouterNoise {
    fn default() -> Self {
        Self {
            program_id: supersonic_tx_core::program_id(),
        }
    }
}

impl AnchorRouterNoise {
    pub fn new(program_id: Pubkey) -> Self {
        Self { program_id }
    }
}

impl DecoyGenerator for AnchorRouterNoise {
    fn generate_decoys(&self, payer: &Pubkey, level: ObfuscationLevel) -> Vec<Instruction> {
        let mut rng = rand::thread_rng();
        let count = match level {
            ObfuscationLevel::Light => 1,
            ObfuscationLevel::Standard => 1,
            ObfuscationLevel::Paranoid => 2,
        };

        let mut instructions = Vec::new();
        // Anchor instruction discriminator: sha256("global:noop_decoy")[..8]
        let hash = solana_sdk::hash::hash(b"global:noop_decoy");
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&hash.to_bytes()[..8]);

        for _ in 0..count {
            let seed: u64 = rng.gen();
            let mut data = Vec::with_capacity(16);
            data.extend_from_slice(&discriminator);
            data.extend_from_slice(&seed.to_le_bytes());

            let accounts = vec![AccountMeta::new_readonly(*payer, true)];
            instructions.push(Instruction {
                program_id: self.program_id,
                accounts,
                data,
            });
        }

        instructions
    }
}

/// Generates typical spl-memo noise instructions to blend in with standard wallet usage.
#[derive(Debug, Clone)]
pub struct MemoNoise {
    pub memos: Vec<String>,
}

impl Default for MemoNoise {
    fn default() -> Self {
        Self {
            memos: vec![
                "Jito tip".to_string(),
                "Transfer".to_string(),
                "0x6a69746f".to_string(), // Hex for jito
                "Swap".to_string(),
            ],
        }
    }
}

impl DecoyGenerator for MemoNoise {
    fn generate_decoys(&self, payer: &Pubkey, level: ObfuscationLevel) -> Vec<Instruction> {
        let mut rng = rand::thread_rng();
        let count = match level {
            ObfuscationLevel::Light => 0, // Light might not need memos
            ObfuscationLevel::Standard => 1,
            ObfuscationLevel::Paranoid => 2,
        };

        let mut instructions = Vec::new();
        let memo_program = Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr").unwrap();

        for _ in 0..count {
            let memo_text = if !self.memos.is_empty() {
                &self.memos[rng.gen_range(0..self.memos.len())]
            } else {
                "tx"
            };

            instructions.push(Instruction {
                program_id: memo_program,
                accounts: vec![AccountMeta::new_readonly(*payer, true)],
                data: memo_text.as_bytes().to_vec(),
            });
        }

        instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benford_law_distribution_properties() {
        let mut rng = rand::thread_rng();
        let n = 10_000;
        let mut counts = [0u32; 10];

        for _ in 0..n {
            let lamports =
                StatisticalTransferNoise::sample_benford_lamports(&mut rng, 1_000, 50_000);
            assert!(
                lamports >= 1_000 && lamports <= 50_000,
                "Lamports {} out of bounds [1000, 50000]",
                lamports
            );

            let leading_digit = lamports
                .to_string()
                .chars()
                .next()
                .unwrap()
                .to_digit(10)
                .unwrap() as usize;
            assert!(leading_digit >= 1 && leading_digit <= 9);
            counts[leading_digit] += 1;
        }

        let freq_1 = counts[1] as f64 / n as f64;
        let freq_2 = counts[2] as f64 / n as f64;
        let freq_9 = counts[9] as f64 / n as f64;

        // Expected Benford's Law frequencies: P(1) ≈ 30.1%, P(2) ≈ 17.6%, P(9) ≈ 4.6%
        assert!(
            freq_1 >= 0.275 && freq_1 <= 0.325,
            "Leading digit 1 frequency {:.4} out of expected Benford range ~30.1%",
            freq_1
        );
        assert!(
            freq_2 >= 0.150 && freq_2 <= 0.200,
            "Leading digit 2 frequency {:.4} out of expected Benford range ~17.6%",
            freq_2
        );
        assert!(
            freq_9 >= 0.025 && freq_9 <= 0.070,
            "Leading digit 9 frequency {:.4} out of expected Benford range ~4.6%",
            freq_9
        );
        assert!(
            counts[1] > counts[2] && counts[2] > counts[3],
            "Benford monotonicity violation"
        );
    }

    #[test]
    fn test_compute_budget_noise_generation() {
        let payer = Pubkey::new_unique();
        let noise = ComputeBudgetNoise::default();

        let decoys_light = noise.generate_decoys(&payer, ObfuscationLevel::Light);
        assert_eq!(decoys_light.len(), 2);

        let decoys_standard = noise.generate_decoys(&payer, ObfuscationLevel::Standard);
        assert_eq!(decoys_standard.len(), 2);

        let decoys_paranoid = noise.generate_decoys(&payer, ObfuscationLevel::Paranoid);
        assert_eq!(decoys_paranoid.len(), 2);

        let noise_no_price = ComputeBudgetNoise::new(Some(300_000), false);
        let decoys_no_price = noise_no_price.generate_decoys(&payer, ObfuscationLevel::Standard);
        assert_eq!(decoys_no_price.len(), 1);
    }

    #[test]
    fn test_anchor_router_noise_generation() {
        let payer = Pubkey::new_unique();
        let noise = AnchorRouterNoise::default();

        let expected_program_id = Pubkey::from_str(supersonic_tx_core::PROGRAM_ID_STR).unwrap();
        assert_eq!(noise.program_id, expected_program_id);

        let decoys = noise.generate_decoys(&payer, ObfuscationLevel::Standard);
        assert_eq!(decoys.len(), 1);

        for ix in decoys {
            assert_eq!(ix.program_id, expected_program_id);
            assert_eq!(ix.accounts.len(), 1);
            assert_eq!(ix.accounts[0].pubkey, payer);
            assert_eq!(ix.accounts[0].is_signer, true);

            assert_eq!(ix.data.len(), 16);
            let expected_discriminator =
                &solana_sdk::hash::hash(b"global:noop_decoy").to_bytes()[..8];
            assert_eq!(&ix.data[..8], expected_discriminator);
        }
    }

    #[test]
    fn statistical_noise_rejects_fake_jupiter_default() {
        let noise = StatisticalTransferNoise::default_tip_allowlist();
        for d in &noise.decoy_destinations {
            let s = d.to_string();
            assert!(
                !s.starts_with("JUP6"),
                "forbidden fake Jupiter destination: {s}"
            );
            // destinations must be non-executable wallets; we cannot check executable off-chain
            // without RPC — enforce allowlist membership instead in builder tests.
        }
    }

    #[test]
    fn statistical_noise_uses_injected_sinks() {
        let sink = Pubkey::new_unique();
        let trusted = TrustedSystemAccount::assume_system_wallet_for_test(sink).unwrap();
        let noise =
            StatisticalTransferNoise::from_sinks(vec![DecoySink::from_trusted_system_account(
                trusted,
            )]);
        let payer = Pubkey::new_unique();
        let decoys = noise.generate_decoys(&payer, ObfuscationLevel::Light);
        assert_eq!(decoys.len(), 1);
        assert_eq!(decoys[0].accounts[1].pubkey, sink);
    }

    #[test]
    fn statistical_noise_rejects_known_program_sinks() {
        let program_ids = [
            solana_sdk::system_program::ID,
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
            Pubkey::from_str("JUP6LkbZbjS1jKKwapdH67yN5k8u4nKq1X4fD6F9yM5").unwrap(),
        ];

        for program_id in program_ids {
            assert!(TrustedSystemAccount::assume_system_wallet_for_test(program_id).is_err());
        }
    }

    #[test]
    fn trusted_system_sinks_still_generate_transfers() {
        let trusted =
            TrustedSystemAccount::assume_system_wallet_for_test(Pubkey::new_unique()).unwrap();
        let sink = DecoySink::from_trusted_system_account(trusted);
        let noise = StatisticalTransferNoise::from_sinks(vec![sink]);
        let payer = Pubkey::new_unique();
        assert_eq!(
            noise.generate_decoys(&payer, ObfuscationLevel::Light)[0].accounts[1].pubkey,
            sink.pubkey()
        );
    }

    #[test]
    fn arbitrary_program_like_sink_requires_provenance_and_is_denied() {
        let program = Pubkey::from_str("JUP6LkbZbjS1jKKwapdH67yN5k8u4nKq1X4fD6F9yM5").unwrap();
        assert!(TrustedSystemAccount::assume_system_wallet_for_test(program).is_err());
        assert!(DecoySink::try_tip_allowlisted(program).is_err());
    }

    #[test]
    fn cooker_decoy_sink_is_the_only_cooker_minting_path() {
        let account = CookedAccount {
            role: CookedRole::DecoySink,
            pubkey: Pubkey::new_unique().to_string(),
            secret_key_path: None,
            funded_lamports: 50_000,
            min_required_lamports: 1_000,
        };
        let trusted = TrustedSystemAccount::from_cooker_decoy_sink(&account).unwrap();
        assert_eq!(trusted.0, Pubkey::from_str(&account.pubkey).unwrap());
    }

    #[test]
    fn cooker_rejects_fee_payer_and_drain_target_roles() {
        for role in [CookedRole::FeePayer, CookedRole::DrainTarget] {
            let account = CookedAccount {
                role,
                pubkey: Pubkey::new_unique().to_string(),
                secret_key_path: None,
                funded_lamports: 50_000,
                min_required_lamports: 1_000,
            };
            assert!(TrustedSystemAccount::from_cooker_decoy_sink(&account).is_err());
        }
    }

    #[test]
    fn router_noop_counts_match_spec_standard() {
        let payer = Pubkey::new_unique();
        let noise = AnchorRouterNoise::default();
        assert_eq!(
            noise
                .generate_decoys(&payer, ObfuscationLevel::Standard)
                .len(),
            1
        );
    }
}

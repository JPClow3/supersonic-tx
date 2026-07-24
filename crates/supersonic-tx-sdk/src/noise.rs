use account_cooker::{CookedAccount, CookedRole};
use rand::Rng;
use solana_client::nonblocking::rpc_client::RpcClient;
#[cfg(test)]
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
pub struct DecoySink {
    account: TrustedSystemAccount,
    rpc_validated: bool,
}

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
    MissingCookedSecretPath,
    InvalidCookedPubkey { value: String },
    RpcValidation { destination: Pubkey, reason: String },
    NotSystemWallet { destination: Pubkey },
    RpcValidationRequired { destination: Pubkey },
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
            Self::MissingCookedSecretPath => {
                write!(
                    f,
                    "cooked DecoySink must include a relative secret_key_path under the cook directory"
                )
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
            Self::RpcValidationRequired { destination } => write!(
                f,
                "decoy sink {destination} must be validated against RPC before atomic use"
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
        TrustedSystemAccount::try_from_tip_allowlist(destination, allowlist).map(|account| Self {
            account,
            rpc_validated: false,
        })
    }

    /// Convert an opaque cooker/system-wallet handoff into a transfer sink.
    pub fn from_trusted_system_account(account: TrustedSystemAccount) -> Self {
        Self {
            account,
            rpc_validated: false,
        }
    }

    pub fn pubkey(self) -> Pubkey {
        self.account.0
    }

    pub(crate) fn require_rpc_validation(self) -> Result<Self, InvalidDecoySink> {
        if self.rpc_validated {
            Ok(self)
        } else {
            Err(InvalidDecoySink::RpcValidationRequired {
                destination: self.pubkey(),
            })
        }
    }

    /// Fetch and prove that a destination is a non-executable, system-owned account.
    pub async fn validate_on_chain(
        rpc: &RpcClient,
        trusted: TrustedSystemAccount,
    ) -> Result<Self, InvalidDecoySink> {
        let destination = trusted.pubkey();
        let account = rpc.get_account(&destination).await.map_err(|error| {
            InvalidDecoySink::RpcValidation {
                destination,
                reason: error.to_string(),
            }
        })?;
        if account.executable || account.owner != solana_sdk::system_program::ID {
            return Err(InvalidDecoySink::NotSystemWallet { destination });
        }
        Ok(Self {
            account: trusted,
            rpc_validated: true,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_rpc_account(
        destination: Pubkey,
        account: &Account,
    ) -> Result<Self, InvalidDecoySink> {
        if account.executable || account.owner != solana_sdk::system_program::ID {
            return Err(InvalidDecoySink::NotSystemWallet { destination });
        }
        TrustedSystemAccount::from_validated_pubkey(destination).map(|account| Self {
            account,
            rpc_validated: true,
        })
    }
}

impl TrustedSystemAccount {
    pub fn pubkey(&self) -> Pubkey {
        self.0
    }

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
        if account.secret_key_path.is_none() {
            return Err(InvalidDecoySink::MissingCookedSecretPath);
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
    pub(crate) fn from_sinks(decoy_destinations: Vec<DecoySink>) -> Self {
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

    pub(crate) fn with_tips(
        mut self,
        tips: &[TrustedSystemAccount],
    ) -> Result<Self, InvalidDecoySink> {
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
        assert!(
            min_lamports <= max_lamports && max_lamports > 0,
            "invalid sampling range"
        );
        loop {
            let u: f64 = rng.gen();
            let leading_digit = ((10.0f64.powf(u)).floor() as u64).clamp(1, 9);
            let mut ranges = Vec::new();
            let mut scale = 1_u64;

            loop {
                if let Some(digit_floor) = leading_digit.checked_mul(scale) {
                    let digit_ceiling = (leading_digit + 1)
                        .checked_mul(scale)
                        .map(|value| value - 1)
                        .unwrap_or(u64::MAX);
                    let lower = digit_floor.max(min_lamports);
                    let upper = digit_ceiling.min(max_lamports);
                    if lower <= upper {
                        ranges.push((lower, upper));
                    }
                }
                match scale.checked_mul(10) {
                    Some(next) if next <= max_lamports => scale = next,
                    _ => break,
                }
            }

            if let Some(&(lower, upper)) = ranges.get(rng.gen_range(0..ranges.len().max(1))) {
                return rng.gen_range(lower..=upper);
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

/// Classic SPL Token program id.
pub const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
/// Token-2022 program id.
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// Token program family used for statistical token-transfer decoys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProgramKind {
    /// Classic SPL Token (`Tokenkeg...`).
    SplToken,
    /// Token-2022 (`TokenzQd...`).
    Token2022,
}

impl TokenProgramKind {
    pub fn program_id(self) -> Pubkey {
        match self {
            Self::SplToken => Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).expect("valid SPL Token id"),
            Self::Token2022 => {
                Pubkey::from_str(TOKEN_2022_PROGRAM_ID).expect("valid Token-2022 id")
            }
        }
    }
}

/// Error returned when a token decoy route fails validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidTokenDecoyRoute {
    ZeroAmountBounds,
    MinExceedsMax { min_amount: u64, max_amount: u64 },
    IdenticalSourceDestination { account: Pubkey },
}

impl fmt::Display for InvalidTokenDecoyRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroAmountBounds => write!(f, "token decoy max_amount must be >= 1"),
            Self::MinExceedsMax {
                min_amount,
                max_amount,
            } => write!(
                f,
                "token decoy min_amount {min_amount} exceeds max_amount {max_amount}"
            ),
            Self::IdenticalSourceDestination { account } => {
                write!(
                    f,
                    "token decoy source and destination are identical: {account}"
                )
            }
        }
    }
}

impl std::error::Error for InvalidTokenDecoyRoute {}

/// Provenance-gated SPL Token / Token-2022 transfer route for decoy traffic.
///
/// Callers must supply funded token accounts they control. This type does not
/// perform RPC mint/ATA validation — that remains an operator / cooker concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenDecoyRoute {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub mint: Pubkey,
    pub decimals: u8,
    pub program: TokenProgramKind,
    pub min_amount: u64,
    pub max_amount: u64,
}

impl TokenDecoyRoute {
    pub fn try_new(
        source: Pubkey,
        destination: Pubkey,
        mint: Pubkey,
        decimals: u8,
        program: TokenProgramKind,
        min_amount: u64,
        max_amount: u64,
    ) -> Result<Self, InvalidTokenDecoyRoute> {
        if max_amount == 0 {
            return Err(InvalidTokenDecoyRoute::ZeroAmountBounds);
        }
        if min_amount == 0 || min_amount > max_amount {
            return Err(InvalidTokenDecoyRoute::MinExceedsMax {
                min_amount,
                max_amount,
            });
        }
        if source == destination {
            return Err(InvalidTokenDecoyRoute::IdenticalSourceDestination { account: source });
        }
        Ok(Self {
            source,
            destination,
            mint,
            decimals,
            program,
            min_amount,
            max_amount,
        })
    }

    /// Build an SPL Token / Token-2022 `TransferChecked` instruction (index 12).
    pub fn transfer_checked(&self, authority: &Pubkey, amount: u64) -> Instruction {
        let mut data = Vec::with_capacity(10);
        data.push(12); // TransferChecked
        data.extend_from_slice(&amount.to_le_bytes());
        data.push(self.decimals);
        Instruction {
            program_id: self.program.program_id(),
            accounts: vec![
                AccountMeta::new(self.source, false),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new(self.destination, false),
                AccountMeta::new_readonly(*authority, true),
            ],
            data,
        }
    }
}

/// Generates statistical SPL Token / Token-2022 micro-transfers as organic DeFi-shaped decoys.
pub struct TokenTransferNoise {
    routes: Vec<TokenDecoyRoute>,
}

impl TokenTransferNoise {
    pub fn from_routes(routes: Vec<TokenDecoyRoute>) -> Self {
        Self { routes }
    }

    pub fn routes(&self) -> &[TokenDecoyRoute] {
        &self.routes
    }
}

impl DecoyGenerator for TokenTransferNoise {
    fn generate_decoys(&self, payer: &Pubkey, level: ObfuscationLevel) -> Vec<Instruction> {
        let mut rng = rand::thread_rng();
        let count = match level {
            ObfuscationLevel::Light => 1,
            ObfuscationLevel::Standard => 2,
            ObfuscationLevel::Paranoid => 4,
        };

        if self.routes.is_empty() {
            return Vec::new();
        }

        let mut instructions = Vec::with_capacity(count);
        for _ in 0..count {
            let route = self.routes[rng.gen_range(0..self.routes.len())];
            let amount = StatisticalTransferNoise::sample_benford_lamports(
                &mut rng,
                route.min_amount,
                route.max_amount,
            );
            instructions.push(route.transfer_checked(payer, amount));
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
            secret_key_path: Some("keys/sink_0.json".into()),
            funded_lamports: 50_000,
            min_required_lamports: 1_000,
        };
        let trusted = TrustedSystemAccount::from_cooker_decoy_sink(&account).unwrap();
        assert_eq!(trusted.0, Pubkey::from_str(&account.pubkey).unwrap());
    }

    #[test]
    fn cooker_decoy_sink_requires_secret_key_path() {
        let account = CookedAccount {
            role: CookedRole::DecoySink,
            pubkey: Pubkey::new_unique().to_string(),
            secret_key_path: None,
            funded_lamports: 50_000,
            min_required_lamports: 1_000,
        };
        assert!(matches!(
            TrustedSystemAccount::from_cooker_decoy_sink(&account),
            Err(InvalidDecoySink::MissingCookedSecretPath)
        ));
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

    #[test]
    fn token_route_builds_transfer_checked_for_spl_and_token2022() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        for program in [TokenProgramKind::SplToken, TokenProgramKind::Token2022] {
            let route =
                TokenDecoyRoute::try_new(source, destination, mint, 6, program, 1_000, 50_000)
                    .unwrap();
            let ix = route.transfer_checked(&authority, 12_345);
            assert_eq!(ix.program_id, program.program_id());
            assert_eq!(ix.data[0], 12);
            assert_eq!(&ix.data[1..9], &12_345u64.to_le_bytes());
            assert_eq!(ix.data[9], 6);
            assert_eq!(ix.accounts[0].pubkey, source);
            assert_eq!(ix.accounts[1].pubkey, mint);
            assert_eq!(ix.accounts[2].pubkey, destination);
            assert_eq!(ix.accounts[3].pubkey, authority);
            assert!(ix.accounts[3].is_signer);
        }
    }

    #[test]
    fn token_transfer_noise_emits_level_counts() {
        let route = TokenDecoyRoute::try_new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            6,
            TokenProgramKind::SplToken,
            1_000,
            50_000,
        )
        .unwrap();
        let noise = TokenTransferNoise::from_routes(vec![route]);
        let payer = Pubkey::new_unique();
        assert_eq!(
            noise.generate_decoys(&payer, ObfuscationLevel::Light).len(),
            1
        );
        assert_eq!(
            noise
                .generate_decoys(&payer, ObfuscationLevel::Standard)
                .len(),
            2
        );
        assert_eq!(
            noise
                .generate_decoys(&payer, ObfuscationLevel::Paranoid)
                .len(),
            4
        );
        assert!(noise
            .generate_decoys(&payer, ObfuscationLevel::Light)
            .iter()
            .all(|ix| ix.program_id == TokenProgramKind::SplToken.program_id()));
    }

    #[test]
    fn token_route_rejects_identical_accounts_and_bad_bounds() {
        let account = Pubkey::new_unique();
        assert!(matches!(
            TokenDecoyRoute::try_new(
                account,
                account,
                Pubkey::new_unique(),
                6,
                TokenProgramKind::Token2022,
                1,
                10
            ),
            Err(InvalidTokenDecoyRoute::IdenticalSourceDestination { .. })
        ));
        assert!(matches!(
            TokenDecoyRoute::try_new(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                6,
                TokenProgramKind::SplToken,
                10,
                5
            ),
            Err(InvalidTokenDecoyRoute::MinExceedsMax { .. })
        ));
    }
}

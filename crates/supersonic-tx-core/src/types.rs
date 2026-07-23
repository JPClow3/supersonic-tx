use serde::{Deserialize, Serialize};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

/// Security level determining decoy density and variance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObfuscationLevel {
    /// Low noise overhead (~1-2 decoy instructions, minimal compute padding)
    Light,
    /// Balanced protection (~3-5 decoys, statistical transfer variance)
    Standard,
    /// Maximum obscurity (High entropy decoy interleaving, ALT integration, maximum padding)
    Paranoid,
}

impl Default for ObfuscationLevel {
    fn default() -> Self {
        Self::Standard
    }
}

/// Represents a decoy instruction pattern designed to fool graph analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecoyKind {
    /// Zero-op CPI call to the supersonic-tx program or recognized protocol program.
    NoopCpi,
    /// Real or simulated micro transfer following Benford's Law distribution.
    StatisticalTransfer {
        destination: Pubkey,
        amount_lamports: u64,
    },
    /// Dynamic Compute Budget Unit padding instruction to equalize TX profiles.
    ComputeBudgetPadding {
        units: u32,
    },
    /// Benign Memo program instruction matching typical dApp interaction signatures.
    ProtocolMemo {
        memo: String,
    },
    /// Custom instruction payload targeting a decoy protocol program.
    Custom {
        program_id: Pubkey,
        data: Vec<u8>,
    },
}

/// Manifest encapsulating real instructions alongside interleaved decoys.
#[derive(Debug, Clone)]
pub struct BundleManifest {
    /// User target instructions (the real intent).
    pub target_instructions: Vec<Instruction>,
    /// Decoy instructions injected to mask intent.
    pub decoy_instructions: Vec<Instruction>,
    /// Target obfuscation security level.
    pub level: ObfuscationLevel,
    /// Interleaved instruction execution order (indices referring to final bundle).
    pub execution_order: Vec<usize>,
}

impl BundleManifest {
    pub fn new(level: ObfuscationLevel) -> Self {
        Self {
            target_instructions: Vec::new(),
            decoy_instructions: Vec::new(),
            level,
            execution_order: Vec::new(),
        }
    }

    /// Total count of all instructions in bundle (real + decoys).
    pub fn len(&self) -> usize {
        self.target_instructions.len() + self.decoy_instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Core errors for supersonic-tx operations.
#[derive(thiserror::Error, Debug)]
pub enum SupersonicError {
    #[error("Transaction size exceeded maximum MTU limit of 1232 bytes (size: {0})")]
    TransactionSizeExceeded(usize),

    #[error("Invalid decoy configuration: {0}")]
    InvalidDecoyConfig(String),

    #[error("Router invocation error: {0}")]
    RouterError(String),

    #[error("Serialization failed: {0}")]
    SerializationError(String),
}

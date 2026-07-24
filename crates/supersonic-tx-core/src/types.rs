use serde::{Deserialize, Serialize};
use solana_sdk::instruction::Instruction;

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

    #[error("Address lookup table fetch failed: {0}")]
    AltFetchFailed(String),

    #[error("Required transaction signature is missing: {0}")]
    MissingSignature(String),

    #[error("Transaction simulation failed: {0}")]
    SimulationFailed(String),

    /// Recent blockhash expired or was never seen by the cluster.
    #[error("RPC blockhash not found")]
    RpcBlockhashNotFound,

    /// Fee payer cannot cover the transaction fee.
    #[error("RPC insufficient funds for fee")]
    RpcInsufficientFundsForFee,

    /// Cluster already processed this signature (often safe to treat as success).
    #[error("RPC transaction already processed")]
    RpcAlreadyProcessed,

    /// Account lock contention / in-flight parallelism conflict.
    #[error("RPC account in use")]
    RpcAccountInUse,

    /// Transport, HTTP, or other transient network-layer RPC failure.
    #[error("RPC transport error: {0}")]
    RpcTransport(String),

    /// Unclassified RPC / client failure (string retained for diagnostics).
    #[error("RPC request failed: {0}")]
    RpcError(String),

    #[error("Fee payer is underfunded: balance {balance}, required at least {required}")]
    Underfunded { balance: u64, required: u64 },

    #[error("Router program is unavailable or not executable: {0}")]
    RouterUnavailable(String),

    #[error("Invalid campaign plan: {0}")]
    InvalidCampaign(String),
}

impl SupersonicError {
    /// Errors where refreshing the blockhash and resubmitting may succeed.
    pub fn is_transient_rpc(&self) -> bool {
        matches!(
            self,
            Self::RpcBlockhashNotFound | Self::RpcAccountInUse | Self::RpcTransport(_)
        )
    }

    /// Map a Solana `TransactionError` into a typed RPC variant when possible.
    pub fn from_transaction_error(error: &solana_sdk::transaction::TransactionError) -> Self {
        use solana_sdk::transaction::TransactionError as TxErr;
        match error {
            TxErr::BlockhashNotFound => Self::RpcBlockhashNotFound,
            TxErr::InsufficientFundsForFee => Self::RpcInsufficientFundsForFee,
            TxErr::AlreadyProcessed => Self::RpcAlreadyProcessed,
            TxErr::AccountInUse => Self::RpcAccountInUse,
            other => Self::RpcError(other.to_string()),
        }
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;
    use solana_sdk::transaction::TransactionError;

    #[test]
    fn classifies_blockhash_and_funds_variants() {
        assert!(matches!(
            SupersonicError::from_transaction_error(&TransactionError::BlockhashNotFound),
            SupersonicError::RpcBlockhashNotFound
        ));
        assert!(matches!(
            SupersonicError::from_transaction_error(&TransactionError::InsufficientFundsForFee),
            SupersonicError::RpcInsufficientFundsForFee
        ));
        assert!(SupersonicError::RpcBlockhashNotFound.is_transient_rpc());
        assert!(!SupersonicError::RpcInsufficientFundsForFee.is_transient_rpc());
        assert!(!SupersonicError::RpcAlreadyProcessed.is_transient_rpc());
    }
}

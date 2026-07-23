use crate::HandoffValidationError;
use serde_json::Error as JsonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CookerError {
    #[error("underfunded handoff: {0}")]
    Underfunded(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("rpc: {0}")]
    Rpc(String),
    #[error("rpc fee estimate was unavailable")]
    FeeEstimateUnavailable,
    #[error("serde: {0}")]
    Serde(String),
    #[error("handoff JSON error: {0}")]
    Json(#[from] JsonError),
    #[error("invalid handoff: {0}")]
    Handoff(#[from] HandoffValidationError),
    #[error("keypair error: {0}")]
    Keypair(String),
    #[error("system clock error: {0}")]
    Clock(#[from] std::time::SystemTimeError),
    #[error("configuration must contain at least one sink")]
    NoSinks,
    #[error("handoff account {account_index} has no secret key path")]
    MissingSecretKeyPath { account_index: usize },
    #[error("handoff account {account_index} keypair does not match pubkey")]
    KeypairMismatch { account_index: usize },
    #[error("refusing to overwrite existing key file: {0}")]
    KeyFileExists(String),
    #[error("generated keypairs do not match handoff account metadata")]
    KeypairMetadataMismatch,
    #[error("funding sponsor does not match cooker and handoff sponsor")]
    SponsorMismatch,
}

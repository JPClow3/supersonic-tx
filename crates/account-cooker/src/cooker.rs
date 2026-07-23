use crate::{CookedAccount, CookedRole, HandoffBundle, HandoffValidationError};
use serde_json::Error as JsonError;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, write_keypair_file, Keypair, Signer},
};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CookerConfig {
    pub cluster: String,
    pub n_sinks: usize,
    pub fund_fee_payer_lamports: u64,
    pub fund_sink_lamports: u64,
    pub min_fee_payer_lamports: u64,
    pub min_sink_lamports: u64,
}

#[derive(Debug)]
pub struct Cooker {
    sponsor_pubkey: Pubkey,
}

#[derive(Debug, Error)]
pub enum CookerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
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
}

impl Cooker {
    pub fn new_offline(sponsor_pubkey: Pubkey) -> Self {
        Self { sponsor_pubkey }
    }

    pub fn generate(
        &self,
        cfg: &CookerConfig,
    ) -> Result<(HandoffBundle, Vec<(CookedRole, Keypair)>), CookerError> {
        if cfg.n_sinks == 0 {
            return Err(CookerError::NoSinks);
        }

        let mut pairs = Vec::with_capacity(cfg.n_sinks + 1);
        pairs.push((CookedRole::FeePayer, Keypair::new()));
        for _ in 0..cfg.n_sinks {
            pairs.push((CookedRole::DecoySink, Keypair::new()));
        }

        let accounts = pairs
            .iter()
            .enumerate()
            .map(|(index, (role, keypair))| {
                let (secret_key_path, funded_lamports, min_required_lamports) = match role {
                    CookedRole::FeePayer => (
                        "keys/fee_payer.json".to_owned(),
                        cfg.fund_fee_payer_lamports,
                        cfg.min_fee_payer_lamports,
                    ),
                    CookedRole::DecoySink => (
                        format!("keys/sink_{}.json", index - 1),
                        cfg.fund_sink_lamports,
                        cfg.min_sink_lamports,
                    ),
                    CookedRole::DrainTarget => (
                        "keys/drain_target.json".to_owned(),
                        0,
                        0,
                    ),
                };
                CookedAccount {
                    role: role.clone(),
                    pubkey: keypair.pubkey().to_string(),
                    secret_key_path: Some(secret_key_path),
                    funded_lamports,
                    min_required_lamports,
                }
            })
            .collect();

        let created_at_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let handoff = HandoffBundle::try_new(
            1,
            cfg.cluster.clone(),
            created_at_unix,
            self.sponsor_pubkey.to_string(),
            accounts,
            Vec::new(),
        )?;
        Ok((handoff, pairs))
    }

    pub fn write_keypair_dir(
        out_dir: &Path,
        pairs: &[(CookedRole, Keypair)],
    ) -> Result<Vec<CookedAccount>, CookerError> {
        let key_dir = out_dir.join("keys");
        fs::create_dir_all(&key_dir)?;

        pairs
            .iter()
            .enumerate()
            .map(|(index, (role, keypair))| {
                let relative_path = keypair_relative_path(role, index);
                let full_path = out_dir.join(&relative_path);
                write_keypair_file(keypair, full_path.to_string_lossy().as_ref())
                    .map_err(|error| CookerError::Keypair(error.to_string()))?;
                Ok(CookedAccount {
                    role: role.clone(),
                    pubkey: keypair.pubkey().to_string(),
                    secret_key_path: Some(relative_path),
                    funded_lamports: 0,
                    min_required_lamports: 0,
                })
            })
            .collect()
    }

    pub fn write_handoff(path: &Path, handoff: &HandoffBundle) -> Result<(), CookerError> {
        handoff.validate()?;
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(handoff)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_handoff(path: &Path) -> Result<HandoffBundle, CookerError> {
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn resolve_keypairs(
        handoff: &HandoffBundle,
        handoff_dir: &Path,
    ) -> Result<Vec<Keypair>, CookerError> {
        handoff.validate()?;
        handoff
            .accounts
            .iter()
            .enumerate()
            .map(|(account_index, account)| {
                let relative_path = account
                    .secret_key_path
                    .as_deref()
                    .ok_or(CookerError::MissingSecretKeyPath { account_index })?;
                let path = handoff_dir.join(relative_path);
                let keypair = read_keypair_file(path.to_string_lossy().as_ref())
                    .map_err(|error| CookerError::Keypair(error.to_string()))?;
                if keypair.pubkey().to_string() != account.pubkey {
                    return Err(CookerError::KeypairMismatch { account_index });
                }
                Ok(keypair)
            })
            .collect()
    }
}

fn keypair_relative_path(role: &CookedRole, index: usize) -> String {
    match role {
        CookedRole::FeePayer => "keys/fee_payer.json".to_owned(),
        CookedRole::DecoySink => format!("keys/sink_{}.json", index - 1),
        CookedRole::DrainTarget => "keys/drain_target.json".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::{Keypair, Signer};

    #[test]
    fn generate_unique_keys_and_write_handoff_paths_only() {
        let dir = tempfile::tempdir().unwrap();
        let sponsor = Keypair::new();
        let cooker = Cooker::new_offline(sponsor.pubkey());
        let cfg = CookerConfig {
            cluster: "devnet".into(),
            n_sinks: 2,
            fund_fee_payer_lamports: 50_000_000,
            fund_sink_lamports: 2_000_000,
            min_fee_payer_lamports: 10_000_000,
            min_sink_lamports: 890_880,
        };
        let (mut handoff, pairs) = cooker.generate(&cfg).unwrap();
        let accounts = Cooker::write_keypair_dir(dir.path(), &pairs).unwrap();
        handoff.accounts = accounts;
        let path = dir.path().join("handoff-1.json");
        Cooker::write_handoff(&path, &handoff).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains(&format!("{:?}", pairs[0].1.to_bytes())));
        assert!(raw.contains("keys/"));
        let loaded = Cooker::load_handoff(&path).unwrap();
        assert_eq!(loaded.schema_version, 1);
        let kps = Cooker::resolve_keypairs(&loaded, dir.path()).unwrap();
        assert_eq!(kps.len(), loaded.accounts.len());
    }
}

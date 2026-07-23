use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookedRole {
    FeePayer,
    DecoySink,
    DrainTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CookedAccount {
    pub role: CookedRole,
    pub pubkey: String,
    pub secret_key_path: Option<String>,
    pub funded_lamports: u64,
    pub min_required_lamports: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffBundle {
    pub schema_version: u32,
    pub cluster: String,
    pub created_at_unix: i64,
    pub sponsor_pubkey: String,
    pub accounts: Vec<CookedAccount>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HandoffValidationError {
    #[error("unsupported handoff schema_version {0}; expected 1")]
    UnsupportedSchemaVersion(u32),
    #[error("account {account_index} has an invalid secret_key_path: {reason}")]
    InvalidSecretKeyPath {
        account_index: usize,
        reason: &'static str,
    },
    #[error("unsupported cluster {0}; expected devnet, mainnet-beta, or localnet")]
    UnsupportedCluster(String),
    #[error("handoff must contain at least one account")]
    EmptyAccounts,
    #[error("handoff must contain exactly one fee payer")]
    InvalidFeePayerCount,
    #[error("invalid sponsor pubkey")]
    InvalidSponsorPubkey,
    #[error("account {account_index} has an invalid pubkey")]
    InvalidAccountPubkey { account_index: usize },
    #[error("account {account_index} duplicates another pubkey")]
    DuplicateAccountPubkey { account_index: usize },
    #[error("account {account_index} requires a secret_key_path")]
    MissingSecretKeyPath { account_index: usize },
    #[error("account {account_index} is underfunded in the handoff")]
    UnderfundedAccount { account_index: usize },
    #[error("created_at_unix is invalid")]
    InvalidCreatedAt,
}

impl HandoffBundle {
    pub fn try_new(
        schema_version: u32,
        cluster: String,
        created_at_unix: i64,
        sponsor_pubkey: String,
        accounts: Vec<CookedAccount>,
        warnings: Vec<String>,
    ) -> Result<Self, HandoffValidationError> {
        let handoff = Self {
            schema_version,
            cluster,
            created_at_unix,
            sponsor_pubkey,
            accounts,
            warnings,
        };
        handoff.validate()?;
        Ok(handoff)
    }

    pub fn validate(&self) -> Result<(), HandoffValidationError> {
        if self.schema_version != 1 {
            return Err(HandoffValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if !matches!(
            self.cluster.as_str(),
            "devnet" | "mainnet-beta" | "localnet"
        ) {
            return Err(HandoffValidationError::UnsupportedCluster(
                self.cluster.clone(),
            ));
        }
        if Pubkey::from_str(&self.sponsor_pubkey).is_err() {
            return Err(HandoffValidationError::InvalidSponsorPubkey);
        }
        if self.accounts.is_empty() {
            return Err(HandoffValidationError::EmptyAccounts);
        }
        if self
            .accounts
            .iter()
            .filter(|account| account.role == CookedRole::FeePayer)
            .count()
            != 1
        {
            return Err(HandoffValidationError::InvalidFeePayerCount);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(i64::MAX);
        if self.created_at_unix <= 0 || self.created_at_unix > now.saturating_add(300) {
            return Err(HandoffValidationError::InvalidCreatedAt);
        }

        let mut pubkeys = HashSet::new();
        for (account_index, account) in self.accounts.iter().enumerate() {
            let pubkey = Pubkey::from_str(&account.pubkey)
                .map_err(|_| HandoffValidationError::InvalidAccountPubkey { account_index })?;
            if !pubkeys.insert(pubkey) {
                return Err(HandoffValidationError::DuplicateAccountPubkey { account_index });
            }
            if account.funded_lamports < account.min_required_lamports {
                return Err(HandoffValidationError::UnderfundedAccount { account_index });
            }
            if matches!(account.role, CookedRole::FeePayer | CookedRole::DecoySink)
                && account.secret_key_path.is_none()
            {
                return Err(HandoffValidationError::MissingSecretKeyPath { account_index });
            }
            if let Some(path) = account.secret_key_path.as_deref() {
                validate_secret_key_path(path).map_err(|reason| {
                    HandoffValidationError::InvalidSecretKeyPath {
                        account_index,
                        reason,
                    }
                })?;
            }
        }

        Ok(())
    }

    pub fn try_from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Serialize for HandoffBundle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        HandoffBundleFields {
            schema_version: self.schema_version,
            cluster: &self.cluster,
            created_at_unix: self.created_at_unix,
            sponsor_pubkey: &self.sponsor_pubkey,
            accounts: &self.accounts,
            warnings: &self.warnings,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HandoffBundle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct HandoffBundleFields {
            schema_version: u32,
            cluster: String,
            created_at_unix: i64,
            sponsor_pubkey: String,
            accounts: Vec<CookedAccount>,
            warnings: Vec<String>,
        }

        let fields = HandoffBundleFields::deserialize(deserializer)?;
        let handoff = Self {
            schema_version: fields.schema_version,
            cluster: fields.cluster,
            created_at_unix: fields.created_at_unix,
            sponsor_pubkey: fields.sponsor_pubkey,
            accounts: fields.accounts,
            warnings: fields.warnings,
        };
        handoff.validate().map_err(serde::de::Error::custom)?;
        Ok(handoff)
    }
}

#[derive(Serialize)]
struct HandoffBundleFields<'a> {
    schema_version: u32,
    cluster: &'a str,
    created_at_unix: i64,
    sponsor_pubkey: &'a str,
    accounts: &'a [CookedAccount],
    warnings: &'a [String],
}

fn validate_secret_key_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("path must not be empty");
    }
    if path.contains('\n') || path.contains('\r') {
        return Err("path must not contain newlines");
    }
    if Path::new(path).is_absolute()
        || path.starts_with('/')
        || path.starts_with('\\')
        || (path.len() >= 2
            && path.as_bytes()[0].is_ascii_alphabetic()
            && path.as_bytes()[1] == b':')
        || path.contains(':')
    {
        return Err("path must be relative");
    }
    if path.split(['/', '\\']).any(|component| component == "..") {
        return Err("path must not contain '..' components");
    }
    if looks_like_embedded_key(path) {
        return Err("path must reference a file, not embedded key material");
    }
    Ok(())
}

fn looks_like_embedded_key(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        if let Ok(bytes) = serde_json::from_str::<Vec<u64>>(trimmed) {
            return bytes.len() >= 16 && bytes.iter().all(|byte| *byte <= u8::MAX as u64);
        }
    }

    trimmed.len() >= 40
        && trimmed.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '=')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_json_round_trip_schema_v1() {
        let h = HandoffBundle {
            schema_version: 1,
            cluster: "devnet".into(),
            created_at_unix: 1721750400,
            sponsor_pubkey: "11111111111111111111111111111111".into(),
            accounts: vec![CookedAccount {
                role: CookedRole::FeePayer,
                pubkey: "GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9".into(),
                secret_key_path: Some("keys/fee_payer.json".into()),
                funded_lamports: 50_000_000,
                min_required_lamports: 10_000_000,
            }],
            warnings: vec![],
        };
        let json = serde_json::to_string_pretty(&h).unwrap();
        assert!(json.contains("\"schema_version\": 1"));
        let json_value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let account = &json_value["accounts"][0];
        assert!(account.get("secret").is_none());
        assert!(account.get("secret_key").is_none());
        let back: HandoffBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn invalid_schema_version_fails_deserialization() {
        let json = r#"{
            "schema_version": 2,
            "cluster": "devnet",
            "created_at_unix": 1721750400,
            "sponsor_pubkey": "11111111111111111111111111111111",
            "accounts": [],
            "warnings": []
        }"#;

        assert!(HandoffBundle::try_from_json(json).is_err());
    }

    #[test]
    fn invalid_structural_bundle_fails_serialization() {
        let handoff = HandoffBundle {
            schema_version: 2,
            cluster: "devnet".into(),
            created_at_unix: 1721750400,
            sponsor_pubkey: "11111111111111111111111111111111".into(),
            accounts: vec![],
            warnings: vec![],
        };

        assert!(serde_json::to_string(&handoff).is_err());
    }

    #[test]
    fn try_new_rejects_invalid_version_and_absolute_path() {
        let version_error = HandoffBundle::try_new(
            2,
            "devnet".into(),
            1721750400,
            "11111111111111111111111111111111".into(),
            vec![],
            vec![],
        )
        .unwrap_err();
        assert_eq!(
            version_error,
            HandoffValidationError::UnsupportedSchemaVersion(2)
        );

        let path_error = HandoffBundle::try_new(
            1,
            "devnet".into(),
            1721750400,
            "11111111111111111111111111111111".into(),
            vec![CookedAccount {
                role: CookedRole::FeePayer,
                pubkey: "GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9".into(),
                secret_key_path: Some("/tmp/keypair.json".into()),
                funded_lamports: 50_000_000,
                min_required_lamports: 10_000_000,
            }],
            vec![],
        )
        .unwrap_err();
        assert_eq!(
            path_error,
            HandoffValidationError::InvalidSecretKeyPath {
                account_index: 0,
                reason: "path must be relative",
            }
        );
    }

    #[test]
    fn absolute_secret_key_path_fails_deserialization() {
        let json = handoff_json_with_path("/tmp/keypair.json");

        assert!(HandoffBundle::try_from_json(&json).is_err());
    }

    #[test]
    fn relative_secret_key_path_is_accepted() {
        let json = handoff_json_with_path("keys/fee_payer.json");

        assert!(HandoffBundle::try_from_json(&json).is_ok());
    }

    #[test]
    fn windows_drive_relative_secret_key_path_fails_deserialization() {
        let json = handoff_json_with_path("C:keys/fee_payer.json");

        assert!(HandoffBundle::try_from_json(&json).is_err());
    }

    #[test]
    fn embedded_secret_material_fails_deserialization() {
        let json = handoff_json_with_path("[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]");

        assert!(HandoffBundle::try_from_json(&json).is_err());
    }

    #[test]
    fn rejects_missing_fee_payer_secret_duplicate_and_underfunding() {
        let payer = "GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9".to_string();
        let mut handoff = HandoffBundle {
            schema_version: 1,
            cluster: "devnet".into(),
            created_at_unix: 1_721_750_400,
            sponsor_pubkey: "11111111111111111111111111111111".into(),
            accounts: vec![CookedAccount {
                role: CookedRole::FeePayer,
                pubkey: payer.clone(),
                secret_key_path: None,
                funded_lamports: 10,
                min_required_lamports: 10,
            }],
            warnings: vec![],
        };
        assert!(matches!(
            handoff.validate(),
            Err(HandoffValidationError::MissingSecretKeyPath { .. })
        ));

        handoff.accounts[0].secret_key_path = Some("keys/payer.json".into());
        handoff.accounts.push(CookedAccount {
            role: CookedRole::DecoySink,
            pubkey: payer,
            secret_key_path: Some("keys/sink.json".into()),
            funded_lamports: 10,
            min_required_lamports: 10,
        });
        assert!(matches!(
            handoff.validate(),
            Err(HandoffValidationError::DuplicateAccountPubkey { .. })
        ));

        handoff.accounts.pop();
        handoff.accounts[0].funded_lamports = 9;
        assert!(matches!(
            handoff.validate(),
            Err(HandoffValidationError::UnderfundedAccount { .. })
        ));
    }

    #[test]
    fn rejects_unsupported_cluster_and_drive_relative_path() {
        let mut json = handoff_json_with_path("C:keypair.json");
        assert!(HandoffBundle::try_from_json(&json).is_err());
        json = json.replace("\"devnet\"", "\"testnet\"");
        assert!(HandoffBundle::try_from_json(&json).is_err());
    }

    fn handoff_json_with_path(path: &str) -> String {
        serde_json::json!({
            "schema_version": 1,
            "cluster": "devnet",
            "created_at_unix": 1721750400,
            "sponsor_pubkey": "11111111111111111111111111111111",
            "accounts": [{
                "role": "FeePayer",
                "pubkey": "GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9",
                "secret_key_path": path,
                "funded_lamports": 50_000_000,
                "min_required_lamports": 10_000_000
            }],
            "warnings": []
        })
        .to_string()
    }
}

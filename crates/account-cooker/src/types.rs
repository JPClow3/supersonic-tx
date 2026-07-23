use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffBundle {
    pub schema_version: u32,
    pub cluster: String,
    pub created_at_unix: i64,
    pub sponsor_pubkey: String,
    pub accounts: Vec<CookedAccount>,
    pub warnings: Vec<String>,
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
                pubkey: "FeePayer111111111111111111111111111111111".into(),
                secret_key_path: Some("keys/fee_payer.json".into()),
                funded_lamports: 50_000_000,
                min_required_lamports: 10_000_000,
            }],
            warnings: vec![],
        };
        let json = serde_json::to_string_pretty(&h).unwrap();
        assert!(json.contains("\"schema_version\": 1"));
        assert!(!json.contains("secret\":"));
        let back: HandoffBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(back, h);
    }
}

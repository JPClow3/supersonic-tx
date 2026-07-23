use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub mod types;

pub use types::*;

/// Program ID for the supersonic-tx on-chain router.
pub const PROGRAM_ID_STR: &str = "GVWCwtjQa1DxxvAD7JFqsdaB65YpouUG3dzdYgsQpvU9";

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID_STR).expect("PROGRAM_ID_STR must be valid")
}

/// Maximum safe payload size for Solana transactions without ALT (Address Lookup Tables).
pub const MAX_TX_PAYLOAD_BYTES: usize = 1232;

#[cfg(test)]
mod program_id_tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    #[test]
    fn program_id_is_not_placeholder() {
        assert_ne!(
            PROGRAM_ID_STR,
            "Super11111111111111111111111111111111111111"
        );
        let pk = Pubkey::from_str(PROGRAM_ID_STR).expect("PROGRAM_ID_STR must be valid base58");
        assert_eq!(pk, program_id());
    }
}

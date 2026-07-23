use anchor_lang::prelude::*;
use supersonic_tx::{
    BundleExecuted, DecoyExecuted, SupersonicProgramError,
};

#[test]
fn test_noop_decoy_event_creation() {
    let authority = Pubkey::new_unique();
    let seed = 42u64;
    let timestamp = 1700000000i64;

    let event = DecoyExecuted {
        authority,
        entropy_seed: seed,
        timestamp,
    };

    assert_eq!(event.authority, authority);
    assert_eq!(event.entropy_seed, 42);
    assert_eq!(event.timestamp, 1700000000);
}

#[test]
fn test_execute_fuzzy_bundle_manifest_validation() {
    let authority = Pubkey::new_unique();
    let bundle_seed = 1001u64;
    let decoy_count_invalid = 0u8;
    let decoy_count_valid = 4u8;

    assert_eq!(decoy_count_invalid == 0, true);
    assert!(decoy_count_valid > 0);

    let event = BundleExecuted {
        authority,
        bundle_seed,
        decoy_count: decoy_count_valid,
        timestamp: 1700000000,
    };

    assert_eq!(event.authority, authority);
    assert_eq!(event.bundle_seed, 1001);
    assert_eq!(event.decoy_count, 4);
}

#[test]
fn test_program_error_variants() {
    let manifest_err = SupersonicProgramError::InvalidBundleManifest;
    let cpi_err = SupersonicProgramError::CpiExecutionFailed;

    assert_ne!(manifest_err, cpi_err);
}

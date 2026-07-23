use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program_test::{processor, BanksClientError, ProgramTest};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Signer,
    system_program,
    transaction::Transaction,
};

const INVALID_BUNDLE_MANIFEST: u32 = 6000;

fn program_test() -> ProgramTest {
    ProgramTest::new(
        "supersonic_tx",
        supersonic_tx_core::program_id(),
        processor!(supersonic_tx::entry),
    )
}

#[tokio::test]
async fn noop_decoy_succeeds() {
    let (mut banks, payer, blockhash) = program_test().start().await;
    let instruction = Instruction {
        program_id: supersonic_tx::id(),
        accounts: supersonic_tx::accounts::NoopDecoy {
            authority: payer.pubkey(),
        }
        .to_account_metas(None),
        data: supersonic_tx::instruction::NoopDecoy { entropy_seed: 42 }.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    banks.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn execute_fuzzy_bundle_rejects_zero_decoys() {
    let (mut banks, payer, blockhash) = program_test().start().await;
    let instruction = Instruction {
        program_id: supersonic_tx::id(),
        accounts: supersonic_tx::accounts::ExecuteFuzzyBundle {
            authority: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: supersonic_tx::instruction::ExecuteFuzzyBundle {
            bundle_seed: 1,
            decoy_count: 0,
            instruction_data: Vec::new(),
        }
        .data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    let error = banks.process_transaction(transaction).await.unwrap_err();
    assert_custom_error(error, INVALID_BUNDLE_MANIFEST);
}

#[tokio::test]
async fn execute_fuzzy_bundle_rejects_missing_cpi_target() {
    let (mut banks, payer, blockhash) = program_test().start().await;
    let instruction = Instruction {
        program_id: supersonic_tx::id(),
        accounts: supersonic_tx::accounts::ExecuteFuzzyBundle {
            authority: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: supersonic_tx::instruction::ExecuteFuzzyBundle {
            bundle_seed: 1,
            decoy_count: 1,
            instruction_data: Vec::new(),
        }
        .data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    assert!(banks.process_transaction(transaction).await.is_err());
}

#[tokio::test]
async fn execute_fuzzy_bundle_system_transfer_cpi() {
    let (mut banks, payer, blockhash) = program_test().start().await;
    let recipient = solana_sdk::pubkey::Pubkey::new_unique();
    let transfer = solana_sdk::system_instruction::transfer(&payer.pubkey(), &recipient, 1);
    let mut accounts = supersonic_tx::accounts::ExecuteFuzzyBundle {
        authority: payer.pubkey(),
        system_program: system_program::ID,
    }
    .to_account_metas(None);
    accounts.extend([
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(recipient, false),
    ]);
    let instruction = Instruction {
        program_id: supersonic_tx::id(),
        accounts,
        data: supersonic_tx::instruction::ExecuteFuzzyBundle {
            bundle_seed: 7,
            decoy_count: 1,
            instruction_data: transfer.data,
        }
        .data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    banks.process_transaction(transaction).await.unwrap();
    assert_eq!(banks.get_balance(recipient).await.unwrap(), 1);
}

fn assert_custom_error(error: BanksClientError, expected_code: u32) {
    let debug = format!("{error:?}");
    assert!(
        debug.contains(&format!("Custom({expected_code})")),
        "expected Anchor custom error {expected_code}, got {debug}"
    );
}

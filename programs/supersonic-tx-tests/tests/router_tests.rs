use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};
use solana_program_test::{processor, BanksClientError, ProgramTest};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    signature::Signer,
    system_program,
    transaction::Transaction,
};

const INVALID_BUNDLE_MANIFEST: u32 = 6000;
const MISSING_CPI_PROGRAM: u32 = 6002;

fn anchor_entry<'info>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'info>],
    instruction_data: &[u8],
) -> ProgramResult {
    // ProgramTest permits the account slice borrow to be shorter than the AccountInfo
    // lifetime, while Anchor's generated entrypoint requires them to match.
    let accounts: &'info [AccountInfo<'info>] = unsafe { std::mem::transmute(accounts) };
    supersonic_tx::entry(program_id, accounts, instruction_data)
}

fn program_test() -> ProgramTest {
    ProgramTest::new(
        "supersonic_tx",
        supersonic_tx_core::program_id(),
        processor!(anchor_entry),
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
    let simulation = banks.simulate_transaction(transaction).await.unwrap();
    assert!(simulation
        .result
        .expect("simulation result missing")
        .is_ok());
    assert!(simulation
        .simulation_details
        .map(|details| details.logs)
        .unwrap_or_default()
        .iter()
        .any(|log| log.contains("executed zero-op decoy instruction")));
}

#[tokio::test]
async fn execute_fuzzy_bundle_rejects_zero_routed_instructions() {
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
            routed_instruction_count: 0,
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
async fn execute_fuzzy_bundle_rejects_multiple_routed_instructions() {
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
            routed_instruction_count: 2,
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
            routed_instruction_count: 1,
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
    assert_custom_error(error, MISSING_CPI_PROGRAM);
}

#[tokio::test]
async fn execute_fuzzy_bundle_rejects_non_executable_cpi_target() {
    let (mut banks, payer, blockhash) = program_test().start().await;
    let mut accounts = supersonic_tx::accounts::ExecuteFuzzyBundle {
        authority: payer.pubkey(),
        system_program: system_program::ID,
    }
    .to_account_metas(None);
    accounts.push(AccountMeta::new_readonly(payer.pubkey(), false));
    let instruction = Instruction {
        program_id: supersonic_tx::id(),
        accounts,
        data: supersonic_tx::instruction::ExecuteFuzzyBundle {
            bundle_seed: 1,
            routed_instruction_count: 1,
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
    assert_custom_error(error, MISSING_CPI_PROGRAM);
}

#[tokio::test]
async fn execute_fuzzy_bundle_system_transfer_cpi() {
    let recipient = Pubkey::new_unique();
    let mut test = program_test();
    test.add_account(
        recipient,
        Account {
            lamports: 1_000_000,
            data: Vec::new(),
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    let (mut banks, payer, blockhash) = test.start().await;
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
            routed_instruction_count: 1,
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
    assert_eq!(banks.get_balance(recipient).await.unwrap(), 1_000_001);
}

#[tokio::test]
async fn execute_fuzzy_bundle_cpi_failure_surfaces_cpi_error() {
    const CPI_EXECUTION_FAILED: u32 = 6001;
    let (mut banks, payer, blockhash) = program_test().start().await;
    // System program is executable, but empty transfer data cannot succeed.
    let mut accounts = supersonic_tx::accounts::ExecuteFuzzyBundle {
        authority: payer.pubkey(),
        system_program: system_program::ID,
    }
    .to_account_metas(None);
    accounts.extend([
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(Pubkey::new_unique(), false),
    ]);
    let instruction = Instruction {
        program_id: supersonic_tx::id(),
        accounts,
        data: supersonic_tx::instruction::ExecuteFuzzyBundle {
            bundle_seed: 9,
            routed_instruction_count: 1,
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
    assert_custom_error(error, CPI_EXECUTION_FAILED);
}

fn assert_custom_error(error: BanksClientError, expected_code: u32) {
    let debug = format!("{error:?}");
    assert!(
        debug.contains(&format!("Custom({expected_code})")),
        "expected Anchor custom error {expected_code}, got {debug}"
    );
}

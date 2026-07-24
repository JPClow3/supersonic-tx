use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::transaction::VersionedTransaction;
use supersonic_tx_core::SupersonicError;

#[derive(Debug, Clone, Copy)]
pub struct SendOptions {
    pub broadcast: bool,
}

pub fn sign_versioned_tx(
    message: VersionedMessage,
    signers: &[&Keypair],
) -> Result<VersionedTransaction, SupersonicError> {
    let transaction = VersionedTransaction::try_new(message, signers)
        .map_err(|error| SupersonicError::MissingSignature(error.to_string()))?;
    assert_fully_signed(&transaction)?;
    Ok(transaction)
}

pub fn assert_fully_signed(transaction: &VersionedTransaction) -> Result<(), SupersonicError> {
    let required = transaction.message.header().num_required_signatures as usize;
    if transaction.signatures.len() != required
        || transaction
            .signatures
            .iter()
            .any(|signature| *signature == Signature::default())
    {
        return Err(SupersonicError::MissingSignature(format!(
            "expected {required} non-default signatures"
        )));
    }
    Ok(())
}

pub async fn verify_executable_program(
    rpc: &RpcClient,
    program_id: &Pubkey,
) -> Result<(), SupersonicError> {
    let account = rpc
        .get_account(program_id)
        .await
        .map_err(|error| SupersonicError::RouterUnavailable(error.to_string()))?;
    let loader_owned = [
        solana_sdk::bpf_loader::ID,
        solana_sdk::bpf_loader_deprecated::ID,
        solana_sdk::bpf_loader_upgradeable::ID,
    ]
    .contains(&account.owner);
    if !account.executable || !loader_owned {
        return Err(SupersonicError::RouterUnavailable(format!(
            "{program_id} is not an executable loader-owned program"
        )));
    }
    Ok(())
}

pub async fn simulate_and_send(
    rpc: &RpcClient,
    transaction: &VersionedTransaction,
    options: SendOptions,
) -> Result<Option<Signature>, SupersonicError> {
    assert_fully_signed(transaction)?;
    let simulation = rpc
        .simulate_transaction(transaction)
        .await
        .map_err(|error| SupersonicError::RpcError(error.to_string()))?;
    if let Some(error) = simulation.value.err {
        let logs = simulation.value.logs.unwrap_or_default().join("\n");
        return Err(SupersonicError::SimulationFailed(format!(
            "{error:?}\n{logs}"
        )));
    }
    if !options.broadcast {
        return Ok(None);
    }
    // Confirm before returning so multi-tx campaigns and post-send drains observe
    // finalized balances instead of racing in-flight spends.
    rpc.send_and_confirm_transaction(transaction)
        .await
        .map(Some)
        .map_err(|error| SupersonicError::RpcError(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::hash::Hash;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Signer;
    use solana_sdk::system_instruction;
    use supersonic_tx_core::ObfuscationLevel;

    use crate::FuzzyBundleBuilder;

    fn message(payer: &Keypair) -> VersionedMessage {
        let instruction = system_instruction::transfer(&payer.pubkey(), &Pubkey::new_unique(), 1);
        FuzzyBundleBuilder::new(payer.pubkey(), ObfuscationLevel::Light)
            .without_transfer_noise()
            .add_target_instruction(instruction)
            .build_versioned_message(Hash::new_unique(), &[])
            .unwrap()
    }

    #[test]
    fn rejects_missing_signer() {
        let payer = Keypair::new();
        assert!(sign_versioned_tx(message(&payer), &[]).is_err());
    }

    #[test]
    fn signed_transaction_has_no_default_signatures() {
        let payer = Keypair::new();
        let transaction = sign_versioned_tx(message(&payer), &[&payer]).unwrap();
        assert_fully_signed(&transaction).unwrap();
    }
}

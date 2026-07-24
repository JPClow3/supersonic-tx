use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::{RpcError, RpcResponseErrorData};
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::transaction::VersionedTransaction;
use supersonic_tx_core::SupersonicError;

#[derive(Debug, Clone, Copy)]
pub struct SendOptions {
    pub broadcast: bool,
}

/// Classify a Solana RPC/client failure into typed [`SupersonicError`] variants.
pub fn classify_client_error(error: ClientError) -> SupersonicError {
    if let Some(tx_err) = error.get_transaction_error() {
        return SupersonicError::from_transaction_error(&tx_err);
    }

    match error.kind() {
        ClientErrorKind::Io(io_err) => SupersonicError::RpcTransport(io_err.to_string()),
        ClientErrorKind::Reqwest(req_err) => SupersonicError::RpcTransport(req_err.to_string()),
        ClientErrorKind::RpcError(RpcError::RpcResponseError {
            data: RpcResponseErrorData::NodeUnhealthy { .. },
            message,
            ..
        }) => SupersonicError::RpcTransport(message.clone()),
        ClientErrorKind::RpcError(rpc_err) => SupersonicError::RpcError(rpc_err.to_string()),
        ClientErrorKind::SerdeJson(serde_err) => SupersonicError::RpcError(serde_err.to_string()),
        ClientErrorKind::SigningError(sign_err) => {
            SupersonicError::MissingSignature(sign_err.to_string())
        }
        // Defensive fallback: `get_transaction_error()` above already intercepts this
        // case in current solana-client, but keep explicit handling in case a future
        // version stops surfacing it there.
        ClientErrorKind::TransactionError(tx_err) => {
            SupersonicError::from_transaction_error(tx_err)
        }
        ClientErrorKind::Custom(message) => {
            // TransportError::Custom often lands here after conversion.
            if message.contains("Blockhash not found") {
                SupersonicError::RpcBlockhashNotFound
            } else if message.contains("Insufficient funds for fee") {
                SupersonicError::RpcInsufficientFundsForFee
            } else if message.contains("already been processed") {
                SupersonicError::RpcAlreadyProcessed
            } else if message.contains("Account in use") {
                SupersonicError::RpcAccountInUse
            } else {
                SupersonicError::RpcError(message.clone())
            }
        }
    }
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
    let account = rpc.get_account(program_id).await.map_err(|error| {
        // Keep the RouterUnavailable surface; classify first for stable messages.
        SupersonicError::RouterUnavailable(classify_client_error(error).to_string())
    })?;
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
        .map_err(classify_client_error)?;
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
        .map_err(classify_client_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::hash::Hash;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Signer;
    use solana_sdk::system_instruction;
    use solana_sdk::transaction::TransactionError;
    use solana_sdk::transport::TransportError;
    use std::io;
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

    #[test]
    fn classify_maps_transaction_errors() {
        let error = ClientError::from(ClientErrorKind::TransactionError(
            TransactionError::BlockhashNotFound,
        ));
        assert!(matches!(
            classify_client_error(error),
            SupersonicError::RpcBlockhashNotFound
        ));

        let error = ClientError::from(ClientErrorKind::TransactionError(
            TransactionError::InsufficientFundsForFee,
        ));
        assert!(matches!(
            classify_client_error(error),
            SupersonicError::RpcInsufficientFundsForFee
        ));
    }

    #[test]
    fn classify_maps_io_to_transport() {
        let error = ClientError::from(ClientErrorKind::Io(io::Error::new(
            io::ErrorKind::ConnectionReset,
            "connection reset",
        )));
        let classified = classify_client_error(error);
        assert!(matches!(classified, SupersonicError::RpcTransport(_)));
        assert!(classified.is_transient_rpc());
    }

    #[test]
    fn classify_maps_transport_transaction_error() {
        let transport = TransportError::TransactionError(TransactionError::AccountInUse);
        let error = ClientError::from(transport);
        assert!(matches!(
            classify_client_error(error),
            SupersonicError::RpcAccountInUse
        ));
    }

    #[test]
    fn classify_maps_custom_message_substrings() {
        let cases = [
            ("Blockhash not found", SupersonicError::RpcBlockhashNotFound),
            (
                "Insufficient funds for fee",
                SupersonicError::RpcInsufficientFundsForFee,
            ),
            (
                "Transaction simulation failed: This transaction has already been processed",
                SupersonicError::RpcAlreadyProcessed,
            ),
            ("Account in use", SupersonicError::RpcAccountInUse),
        ];
        for (message, expected) in cases {
            let error = ClientError::from(ClientErrorKind::Custom(message.to_string()));
            let classified = classify_client_error(error);
            assert_eq!(
                std::mem::discriminant(&classified),
                std::mem::discriminant(&expected),
                "message {message:?} classified as {classified:?}, expected {expected:?}"
            );
        }
    }

    #[test]
    fn classify_maps_unrecognized_custom_message_to_generic_rpc_error() {
        let error = ClientError::from(ClientErrorKind::Custom("some new RPC wording".to_string()));
        assert!(matches!(
            classify_client_error(error),
            SupersonicError::RpcError(message) if message == "some new RPC wording"
        ));
    }
}

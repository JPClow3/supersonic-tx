use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::account::Account;
use solana_sdk::address_lookup_table::{
    program as address_lookup_table_program, state::AddressLookupTable, AddressLookupTableAccount,
};
use solana_sdk::pubkey::Pubkey;
use supersonic_tx_core::SupersonicError;

pub struct AltResolver;

impl AltResolver {
    pub async fn fetch(
        rpc: &RpcClient,
        address: &Pubkey,
    ) -> Result<AddressLookupTableAccount, SupersonicError> {
        let account = rpc
            .get_account(address)
            .await
            .map_err(|error| SupersonicError::AltFetchFailed(error.to_string()))?;
        Self::from_account(address, &account)
    }

    pub fn from_account(
        address: &Pubkey,
        account: &Account,
    ) -> Result<AddressLookupTableAccount, SupersonicError> {
        if account.owner != address_lookup_table_program::id() {
            return Err(SupersonicError::AltFetchFailed(format!(
                "{address} is not owned by the address lookup table program"
            )));
        }
        let table = AddressLookupTable::deserialize(&account.data)
            .map_err(|error| SupersonicError::AltFetchFailed(error.to_string()))?;
        Ok(AddressLookupTableAccount {
            key: *address,
            addresses: table.addresses.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_alt_owner_and_empty_data() {
        let address = Pubkey::new_unique();
        let wrong_owner = Account {
            lamports: 1,
            data: Vec::new(),
            owner: solana_sdk::system_program::ID,
            executable: false,
            rent_epoch: 0,
        };
        assert!(AltResolver::from_account(&address, &wrong_owner).is_err());

        let empty_alt = Account {
            owner: address_lookup_table_program::id(),
            ..wrong_owner
        };
        assert!(AltResolver::from_account(&address, &empty_alt).is_err());
    }
}

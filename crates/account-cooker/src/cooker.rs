use crate::{CookedAccount, CookedRole, CookerError, HandoffBundle};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    message::Message,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

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

        let mut sink_index = 0;
        let accounts = pairs
            .iter()
            .map(|(role, keypair)| {
                let (secret_key_path, funded_lamports, min_required_lamports) = match role {
                    CookedRole::FeePayer => (
                        "keys/fee_payer.json".to_owned(),
                        cfg.fund_fee_payer_lamports,
                        cfg.min_fee_payer_lamports,
                    ),
                    CookedRole::DecoySink => (
                        keypair_relative_path(role, &mut sink_index),
                        cfg.fund_sink_lamports,
                        cfg.min_sink_lamports,
                    ),
                    CookedRole::DrainTarget => ("keys/drain_target.json".to_owned(), 0, 0),
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
        accounts: &[CookedAccount],
    ) -> Result<Vec<CookedAccount>, CookerError> {
        if pairs.len() != accounts.len()
            || pairs
                .iter()
                .zip(accounts)
                .any(|((role, keypair), account)| {
                    role != &account.role || keypair.pubkey().to_string() != account.pubkey
                })
        {
            return Err(CookerError::KeypairMetadataMismatch);
        }

        let key_dir = out_dir.join("keys");
        fs::create_dir_all(&key_dir)?;

        let mut sink_index = 0;
        let writes = pairs
            .iter()
            .map(|(role, keypair)| {
                let relative_path = keypair_relative_path(role, &mut sink_index);
                let full_path = out_dir.join(&relative_path);
                if full_path.exists() {
                    return Err(CookerError::KeyFileExists(full_path.display().to_string()));
                }
                Ok((relative_path, full_path, keypair))
            })
            .collect::<Result<Vec<_>, CookerError>>()?;

        let mut created = Vec::new();
        for (_, full_path, keypair) in &writes {
            if let Err(error) = write_keypair_file_new(full_path, keypair) {
                for path in created {
                    let _ = fs::remove_file(path);
                }
                return Err(error);
            }
            created.push(full_path);
        }

        Ok(accounts
            .iter()
            .zip(writes)
            .map(|(account, (relative_path, _, _))| {
                let mut account = account.clone();
                account.secret_key_path = Some(relative_path);
                account
            })
            .collect())
    }

    pub fn write_handoff(path: &Path, handoff: &HandoffBundle) -> Result<(), CookerError> {
        handoff.validate()?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
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
    ) -> Result<Vec<(usize, Keypair)>, CookerError> {
        handoff.validate()?;
        handoff
            .accounts
            .iter()
            .enumerate()
            .filter_map(|(account_index, account)| {
                if account.role == CookedRole::DrainTarget && account.secret_key_path.is_none() {
                    None
                } else {
                    Some(
                        resolve_keypair(account, account_index, handoff_dir)
                            .map(|keypair| (account_index, keypair)),
                    )
                }
            })
            .collect()
    }

    pub async fn fund_accounts(
        &self,
        rpc: &RpcClient,
        sponsor: &Keypair,
        handoff: &HandoffBundle,
        _handoff_dir: &Path,
    ) -> Result<(), CookerError> {
        handoff.validate()?;
        let handoff_sponsor = handoff
            .sponsor_pubkey
            .parse::<Pubkey>()
            .map_err(|_| CookerError::SponsorMismatch)?;
        if sponsor.pubkey() != self.sponsor_pubkey || sponsor.pubkey() != handoff_sponsor {
            return Err(CookerError::SponsorMismatch);
        }

        for account in &handoff.accounts {
            if account.funded_lamports == 0 {
                continue;
            }
            let destination = account
                .pubkey
                .parse::<Pubkey>()
                .map_err(|error| CookerError::Rpc(format!("invalid account pubkey: {error}")))?;
            // Refresh after each confirm so large sink batches cannot expire mid-fund.
            let blockhash = rpc
                .get_latest_blockhash()
                .await
                .map_err(|error| CookerError::Rpc(error.to_string()))?;
            let instruction = system_instruction::transfer(
                &sponsor.pubkey(),
                &destination,
                account.funded_lamports,
            );
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&sponsor.pubkey()),
                &[sponsor],
                blockhash,
            );
            rpc.send_and_confirm_transaction(&transaction)
                .await
                .map_err(|error| CookerError::Rpc(error.to_string()))?;
        }
        Ok(())
    }

    pub async fn drain(
        &self,
        rpc: &RpcClient,
        handoff: &HandoffBundle,
        handoff_dir: &Path,
        destination: &Pubkey,
    ) -> Result<(), CookerError> {
        handoff.validate()?;
        let rent_exempt_minimum = rpc
            .get_minimum_balance_for_rent_exemption(0)
            .await
            .map_err(|error| CookerError::Rpc(error.to_string()))?;
        for (account_index, account) in handoff.accounts.iter().enumerate() {
            if should_skip_drain(account) {
                continue;
            }
            let keypair = resolve_keypair(account, account_index, handoff_dir)?;
            let balance = rpc
                .get_balance(&keypair.pubkey())
                .await
                .map_err(|error| CookerError::Rpc(error.to_string()))?;
            let blockhash = rpc
                .get_latest_blockhash()
                .await
                .map_err(|error| CookerError::Rpc(error.to_string()))?;
            let mut fee_message = Message::new(
                &[system_instruction::transfer(
                    &keypair.pubkey(),
                    destination,
                    0,
                )],
                Some(&keypair.pubkey()),
            );
            fee_message.recent_blockhash = blockhash;
            let fee = rpc
                .get_fee_for_message(&fee_message)
                .await
                .map_err(|error| CookerError::Rpc(error.to_string()))?;
            let amount = drain_amount(balance, fee, rent_exempt_minimum);
            if amount == 0 {
                continue;
            }
            let transaction = Transaction::new_signed_with_payer(
                &[system_instruction::transfer(
                    &keypair.pubkey(),
                    destination,
                    amount,
                )],
                Some(&keypair.pubkey()),
                &[&keypair],
                blockhash,
            );
            rpc.send_and_confirm_transaction(&transaction)
                .await
                .map_err(|error| CookerError::Rpc(error.to_string()))?;
        }
        Ok(())
    }

    pub fn assert_funded_for_cast(
        handoff: &HandoffBundle,
        estimated_fees: u64,
    ) -> Result<(), CookerError> {
        let mut shortfalls = Vec::new();
        for account in &handoff.accounts {
            let required = account.min_required_lamports.saturating_add(
                if account.role == CookedRole::FeePayer {
                    estimated_fees
                } else {
                    0
                },
            );
            if account.funded_lamports < required {
                shortfalls.push(format!(
                    "{} shortfall {} lamports",
                    account.pubkey,
                    required - account.funded_lamports
                ));
            }
        }
        if shortfalls.is_empty() {
            Ok(())
        } else {
            Err(CookerError::Underfunded(shortfalls.join(", ")))
        }
    }

    pub fn detect_reuse_warnings(out_dir: &Path, pubkeys: &[Pubkey]) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut seen = HashSet::new();
        for pubkey in pubkeys {
            if !seen.insert(*pubkey) {
                let warning = format!("pubkey reuse detected: {pubkey} appears more than once");
                eprintln!("{warning}");
                warnings.push(warning);
            }
        }

        let key_dir = out_dir.join("keys");
        let Ok(entries) = fs::read_dir(&key_dir) else {
            return warnings;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Ok(keypair) = read_keypair_file(path.to_string_lossy().as_ref()) else {
                continue;
            };
            if pubkeys.contains(&keypair.pubkey()) {
                let warning = format!("pubkey reuse detected in {}", path.display());
                eprintln!("{warning}");
                warnings.push(warning);
            }
        }
        warnings
    }
}

fn resolve_keypair(
    account: &CookedAccount,
    account_index: usize,
    handoff_dir: &Path,
) -> Result<Keypair, CookerError> {
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
}

fn drain_amount(balance: u64, fee: u64, rent_exempt_minimum: u64) -> u64 {
    balance
        .saturating_sub(fee)
        .saturating_sub(rent_exempt_minimum)
}

fn should_skip_drain(account: &CookedAccount) -> bool {
    account.role == CookedRole::DrainTarget
}

fn keypair_relative_path(role: &CookedRole, sink_index: &mut usize) -> String {
    match role {
        CookedRole::FeePayer => "keys/fee_payer.json".to_owned(),
        CookedRole::DecoySink => {
            let path = format!("keys/sink_{}.json", *sink_index);
            *sink_index += 1;
            path
        }
        CookedRole::DrainTarget => "keys/drain_target.json".to_owned(),
    }
}

fn write_keypair_file_new(path: &Path, keypair: &Keypair) -> Result<(), CookerError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            CookerError::KeyFileExists(path.display().to_string())
        } else {
            CookerError::Io(error)
        }
    })?;
    let encoded = serde_json::to_vec(&keypair.to_bytes().to_vec())
        .map_err(|error| CookerError::Keypair(error.to_string()))?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    Ok(())
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
        let accounts = Cooker::write_keypair_dir(dir.path(), &pairs, &handoff.accounts).unwrap();
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

    #[test]
    fn write_keypair_dir_handles_sink_first_ordering() {
        let dir = tempfile::tempdir().unwrap();
        let pairs = vec![
            (CookedRole::DecoySink, Keypair::new()),
            (CookedRole::FeePayer, Keypair::new()),
            (CookedRole::DecoySink, Keypair::new()),
        ];

        let source_accounts = pairs
            .iter()
            .map(|(role, keypair)| CookedAccount {
                role: role.clone(),
                pubkey: keypair.pubkey().to_string(),
                secret_key_path: None,
                funded_lamports: 9,
                min_required_lamports: 7,
            })
            .collect::<Vec<_>>();
        let accounts = Cooker::write_keypair_dir(dir.path(), &pairs, &source_accounts).unwrap();

        assert_eq!(
            accounts
                .iter()
                .map(|account| account.secret_key_path.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("keys/sink_0.json"),
                Some("keys/fee_payer.json"),
                Some("keys/sink_1.json"),
            ]
        );
        assert!(accounts
            .iter()
            .all(|account| account.funded_lamports == 9 && account.min_required_lamports == 7));
    }

    #[test]
    fn second_write_refuses_to_change_existing_key_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let sponsor = Keypair::new();
        let cooker = Cooker::new_offline(sponsor.pubkey());
        let cfg = CookerConfig {
            cluster: "devnet".into(),
            n_sinks: 1,
            fund_fee_payer_lamports: 50,
            fund_sink_lamports: 2,
            min_fee_payer_lamports: 10,
            min_sink_lamports: 0,
        };
        let (handoff, first_pairs) = cooker.generate(&cfg).unwrap();
        Cooker::write_keypair_dir(dir.path(), &first_pairs, &handoff.accounts).unwrap();
        let payer_path = dir.path().join("keys/fee_payer.json");
        let before = fs::read(&payer_path).unwrap();

        let (second_handoff, second_pairs) = cooker.generate(&cfg).unwrap();
        let error = Cooker::write_keypair_dir(dir.path(), &second_pairs, &second_handoff.accounts)
            .unwrap_err();

        assert!(matches!(error, CookerError::KeyFileExists(_)));
        assert_eq!(fs::read(payer_path).unwrap(), before);
    }

    #[test]
    fn assert_funded_refuses_shortfall() {
        let handoff = HandoffBundle {
            schema_version: 1,
            cluster: "devnet".into(),
            created_at_unix: 1,
            sponsor_pubkey: "x".into(),
            accounts: vec![CookedAccount {
                role: CookedRole::FeePayer,
                pubkey: "p".into(),
                secret_key_path: Some("keys/fee_payer.json".into()),
                funded_lamports: 1_000,
                min_required_lamports: 10_000_000,
            }],
            warnings: vec![],
        };
        let err = Cooker::assert_funded_for_cast(&handoff, 5_000).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("shortfall") || msg.contains("underfunded"));
    }

    #[test]
    fn detect_reuse_warns_on_duplicate_pubkey() {
        let pk = Pubkey::new_unique();
        let warnings = Cooker::detect_reuse_warnings(Path::new("/tmp"), &[pk, pk]);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn drain_amount_preserves_rent_exempt_minimum() {
        assert_eq!(drain_amount(10_000, 500, 2_000), 7_500);
        assert_eq!(drain_amount(2_500, 500, 2_000), 0);
        assert_eq!(drain_amount(2_000, 500, 2_000), 0);
    }

    #[test]
    fn pathless_drain_target_is_skipped_before_keypair_resolution() {
        let target = CookedAccount {
            role: CookedRole::DrainTarget,
            pubkey: Pubkey::new_unique().to_string(),
            secret_key_path: None,
            funded_lamports: 0,
            min_required_lamports: 0,
        };
        assert!(target.secret_key_path.is_none());
        assert!(should_skip_drain(&target));
    }

    #[tokio::test]
    async fn funding_rejects_mismatched_sponsor_before_rpc() {
        let configured = Keypair::new();
        let actual = Keypair::new();
        let cooker = Cooker::new_offline(configured.pubkey());
        let cfg = CookerConfig {
            cluster: "devnet".into(),
            n_sinks: 1,
            fund_fee_payer_lamports: 1,
            fund_sink_lamports: 1,
            min_fee_payer_lamports: 0,
            min_sink_lamports: 0,
        };
        let (handoff, _) = cooker.generate(&cfg).unwrap();
        let rpc = RpcClient::new("http://127.0.0.1:1".to_owned());

        let error = cooker
            .fund_accounts(&rpc, &actual, &handoff, Path::new("."))
            .await
            .unwrap_err();

        assert!(matches!(error, CookerError::SponsorMismatch));
    }
}

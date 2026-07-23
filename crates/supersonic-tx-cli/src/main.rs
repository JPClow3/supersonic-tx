use account_cooker::{CookedRole, Cooker, CookerConfig, HandoffBundle};
use anchor_lang::{InstructionData, ToAccountMetas};
use clap::{Parser, Subcommand, ValueEnum};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use solana_sdk::{system_instruction, system_instruction::SystemInstruction, system_program};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use supersonic_tx_core::{ObfuscationLevel, SupersonicError, MAX_TX_PAYLOAD_BYTES};
use supersonic_tx_sdk::{
    sign_versioned_tx, simulate_and_send, verify_executable_program, AltResolver, CampaignPlanner,
    DecoySink, FuzzyBundleBuilder, PlannedTxKind, SendOptions, TrustedSystemAccount,
};

const DEFAULT_RPC_URL: &str = "https://api.devnet.solana.com";
const CONSERVATIVE_FEE_AND_DECOY_BUDGET: u64 = 255_000;
const DEVNET_GENESIS_HASH: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1";
const MAINNET_GENESIS_HASH: &str = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp";

#[derive(Parser, Debug)]
#[command(
    name = "supersonic-tx",
    version,
    about = "Behavioral-obscurity transaction tooling for Solana"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliLevel {
    Light,
    Standard,
    Paranoid,
}

impl From<CliLevel> for ObfuscationLevel {
    fn from(value: CliLevel) -> Self {
        match value {
            CliLevel::Light => Self::Light,
            CliLevel::Standard => Self::Standard,
            CliLevel::Paranoid => Self::Paranoid,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    Cook {
        #[arg(long)]
        sponsor_keypair: PathBuf,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        rpc_url: String,
        #[arg(long, default_value = "devnet")]
        cluster: String,
        #[arg(long, default_value_t = 2)]
        sinks: usize,
        #[arg(long, default_value_t = 50_000_000)]
        fee_payer_lamports: u64,
        #[arg(long, default_value_t = 2_000_000)]
        sink_lamports: u64,
        /// Generate keypairs and handoff without funding; fund before casting.
        #[arg(long)]
        dry_run: bool,
    },
    /// Assemble an unsigned, offline V0 message and print diagnostics.
    Assemble {
        #[arg(long, value_enum, default_value_t = CliLevel::Standard)]
        level: CliLevel,
        #[arg(long)]
        payer: Option<String>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long, default_value_t = 100_000)]
        amount: u64,
    },
    /// Sign and run simulateTransaction. Never broadcasts.
    Simulate {
        #[arg(long, value_enum, default_value_t = CliLevel::Standard)]
        level: CliLevel,
        #[arg(long)]
        target: String,
        #[arg(long, default_value_t = 100_000)]
        amount: u64,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        rpc_url: String,
        #[arg(long)]
        keypair: Option<PathBuf>,
        #[arg(long)]
        handoff: Option<PathBuf>,
        #[arg(long)]
        alt: Option<String>,
        #[arg(long)]
        tip: Vec<String>,
        #[arg(long)]
        via_router: bool,
    },
    /// Sign, simulate, and optionally submit one atomic transaction.
    Cast {
        #[arg(long)]
        target: String,
        #[arg(long, default_value_t = 100_000)]
        amount: u64,
        #[arg(long, value_enum, default_value_t = CliLevel::Standard)]
        level: CliLevel,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        rpc_url: String,
        #[arg(long)]
        keypair: Option<PathBuf>,
        #[arg(long)]
        handoff: Option<PathBuf>,
        #[arg(long)]
        alt: Option<String>,
        #[arg(long)]
        tip: Vec<String>,
        #[arg(long)]
        via_router: bool,
        #[arg(long)]
        send: bool,
    },
    Campaign {
        #[arg(long)]
        target: String,
        #[arg(long, default_value_t = 100_000)]
        amount: u64,
        #[arg(long, value_enum, default_value_t = CliLevel::Standard)]
        level: CliLevel,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        rpc_url: String,
        #[arg(long)]
        keypair: Option<PathBuf>,
        #[arg(long)]
        handoff: Option<PathBuf>,
        #[arg(long)]
        alt: Option<String>,
        #[arg(long)]
        tip: Vec<String>,
        #[arg(long, default_value_t = 2)]
        txs: usize,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        isolate_intent: bool,
        #[arg(long)]
        send: bool,
        /// Drain cooked accounts after a successfully broadcast real intent.
        #[arg(long)]
        drain_to: Option<String>,
    },
    Info,
}

struct LoadedAccounts {
    payer: Keypair,
    sinks: Vec<DecoySink>,
    handoff: Option<(HandoffBundle, PathBuf)>,
}

async fn load_accounts(
    rpc: &RpcClient,
    keypair_path: Option<&Path>,
    handoff_path: Option<&Path>,
    tips: &[String],
) -> Result<LoadedAccounts, Box<dyn std::error::Error>> {
    if keypair_path.is_some() == handoff_path.is_some() {
        return Err("provide exactly one of --keypair or --handoff".into());
    }

    let (payer, mut sinks, loaded_handoff) = if let Some(path) = handoff_path {
        let handoff = Cooker::load_handoff(path)?;
        verify_rpc_cluster(rpc, &handoff.cluster, "").await?;
        Cooker::assert_funded_for_cast(&handoff, CONSERVATIVE_FEE_AND_DECOY_BUDGET)?;
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let keypairs = Cooker::resolve_keypairs(&handoff, directory)?;
        let mut payer = None;
        let mut sinks = Vec::new();
        for (account_index, keypair) in keypairs {
            let account = &handoff.accounts[account_index];
            match account.role {
                CookedRole::FeePayer => payer = Some(keypair),
                CookedRole::DecoySink => {
                    let trusted = TrustedSystemAccount::from_cooker_decoy_sink(account)?;
                    sinks.push(DecoySink::validate_on_chain(rpc, trusted).await?);
                }
                CookedRole::DrainTarget => {}
            }
        }
        (
            payer.ok_or("handoff has no fee payer keypair")?,
            sinks,
            Some((handoff, directory.to_path_buf())),
        )
    } else {
        (
            read_keypair_file(keypair_path.expect("checked exclusive input"))?,
            Vec::new(),
            None,
        )
    };

    let tip_allowlist = tips
        .iter()
        .map(|tip| Pubkey::from_str(tip))
        .collect::<Result<Vec<_>, _>>()?;
    for destination in &tip_allowlist {
        let trusted = TrustedSystemAccount::try_from_tip_allowlist(*destination, &tip_allowlist)?;
        sinks.push(DecoySink::validate_on_chain(rpc, trusted).await?);
    }
    Ok(LoadedAccounts {
        payer,
        sinks,
        handoff: loaded_handoff,
    })
}

async fn verify_rpc_cluster(
    rpc: &RpcClient,
    declared_cluster: &str,
    rpc_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let genesis = rpc.get_genesis_hash().await?.to_string();
    let matches = match declared_cluster {
        "devnet" => genesis == DEVNET_GENESIS_HASH,
        "mainnet-beta" => genesis == MAINNET_GENESIS_HASH,
        "localnet" => {
            genesis != DEVNET_GENESIS_HASH
                && genesis != MAINNET_GENESIS_HASH
                && (rpc_url.is_empty()
                    || rpc_url.contains("localhost")
                    || rpc_url.contains("127.0.0.1"))
        }
        _ => false,
    };
    if !matches {
        return Err(format!(
            "RPC genesis hash {genesis} does not match declared cluster {declared_cluster}"
        )
        .into());
    }
    Ok(())
}

async fn resolve_alt(
    rpc: &RpcClient,
    alt: Option<&str>,
) -> Result<
    Vec<solana_sdk::address_lookup_table::AddressLookupTableAccount>,
    Box<dyn std::error::Error>,
> {
    let Some(alt) = alt else {
        return Ok(Vec::new());
    };
    let address = Pubkey::from_str(alt)?;
    match AltResolver::fetch(rpc, &address).await {
        Ok(table) => Ok(vec![table]),
        Err(error) => {
            eprintln!("ALT unavailable ({error}); falling back to non-ALT V0 compilation");
            Ok(Vec::new())
        }
    }
}

async fn build_signed(
    rpc: &RpcClient,
    accounts: &LoadedAccounts,
    target: Pubkey,
    amount: u64,
    level: ObfuscationLevel,
    alt: Option<&str>,
    via_router: bool,
) -> Result<(solana_sdk::transaction::VersionedTransaction, usize, usize), Box<dyn std::error::Error>>
{
    let payer = accounts.payer.pubkey();
    let direct_target = system_instruction::transfer(&payer, &target, amount);
    let target_instruction = if via_router {
        let router = supersonic_tx_core::program_id();
        verify_executable_program(rpc, &router).await?;
        routed_instruction(payer, direct_target)
    } else {
        direct_target
    };
    let mut builder =
        FuzzyBundleBuilder::new(payer, level).add_target_instruction(target_instruction);
    builder = if accounts.sinks.is_empty() {
        eprintln!("Transfer noise disabled: no RPC-validated system-wallet sinks were supplied");
        builder.without_transfer_noise()
    } else {
        builder.with_sinks(accounts.sinks.clone())?
    };
    let tables = resolve_alt(rpc, alt).await?;
    let blockhash = rpc.get_latest_blockhash().await?;
    let built = builder.build_bundle(blockhash, &tables)?;
    let fee = match &built.message {
        VersionedMessage::Legacy(message) => rpc.get_fee_for_message(message).await?,
        VersionedMessage::V0(message) => rpc.get_fee_for_message(message).await?,
    };
    let balance = rpc.get_balance(&payer).await?;
    let required = amount
        .saturating_add(fee)
        .saturating_add(if accounts.sinks.is_empty() {
            0
        } else {
            250_000
        });
    if balance < required {
        return Err(Box::new(SupersonicError::Underfunded { balance, required })
            as Box<dyn std::error::Error>);
    }
    let decoy_count = built.manifest.decoy_instructions.len();
    let transaction = sign_versioned_tx(built.message, &[&accounts.payer])?;
    Ok((transaction, built.serialized_size, decoy_count))
}

fn routed_instruction(authority: Pubkey, target: Instruction) -> Instruction {
    let mut accounts = supersonic_tx::accounts::ExecuteFuzzyBundle {
        authority,
        system_program: system_program::ID,
    }
    .to_account_metas(None);
    accounts.push(AccountMeta::new_readonly(target.program_id, false));
    accounts.extend(target.accounts);
    Instruction {
        program_id: supersonic_tx_core::program_id(),
        accounts,
        data: supersonic_tx::instruction::ExecuteFuzzyBundle {
            bundle_seed: 0,
            routed_instruction_count: 1,
            instruction_data: target.data,
        }
        .data(),
    }
}

fn transfer_spend(instructions: &[Instruction], payer: &Pubkey) -> u64 {
    instructions
        .iter()
        .filter(|instruction| {
            instruction.program_id == system_program::ID
                && instruction
                    .accounts
                    .first()
                    .is_some_and(|account| account.pubkey == *payer)
        })
        .filter_map(|instruction| {
            bincode::deserialize::<SystemInstruction>(&instruction.data)
                .ok()
                .and_then(|instruction| match instruction {
                    SystemInstruction::Transfer { lamports } => Some(lamports),
                    _ => None,
                })
        })
        .fold(0_u64, u64::saturating_add)
}

fn decoy_preserves_reserve(balance: u64, real_reserve: u64, spend: u64, fee: u64) -> bool {
    balance >= real_reserve.saturating_add(spend).saturating_add(fee)
}

async fn message_fee(
    rpc: &RpcClient,
    message: &VersionedMessage,
) -> Result<u64, Box<dyn std::error::Error>> {
    Ok(match message {
        VersionedMessage::Legacy(message) => rpc.get_fee_for_message(message).await?,
        VersionedMessage::V0(message) => rpc.get_fee_for_message(message).await?,
    })
}

fn run_assemble(
    level: CliLevel,
    payer: Option<String>,
    target: Option<String>,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let payer = payer
        .as_deref()
        .map(Pubkey::from_str)
        .transpose()?
        .unwrap_or_else(Pubkey::new_unique);
    let mut builder = FuzzyBundleBuilder::new(payer, level.into()).without_transfer_noise();
    if let Some(target) = target {
        builder = builder.add_target_instruction(system_instruction::transfer(
            &payer,
            &Pubkey::from_str(&target)?,
            amount,
        ));
    }
    let built = builder.build_bundle(Hash::default(), &[])?;
    let total = built.manifest.len();
    let decoys = built.manifest.decoy_instructions.len();
    let ratio = if total == 0 {
        0.0
    } else {
        decoys as f64 / total as f64
    };
    println!(
        "Unsigned assembly: {}/{MAX_TX_PAYLOAD_BYTES} bytes ({:.1}% MTU), {decoys}/{total} decoys ({ratio:.2} ratio)",
        built.serialized_size,
        built.serialized_size as f64 * 100.0 / MAX_TX_PAYLOAD_BYTES as f64
    );
    println!("CU diagnostics: compute-budget profile included; Benford: not applicable without transfer noise");
    Ok(())
}

async fn run_cast(
    target: String,
    amount: u64,
    level: CliLevel,
    rpc_url: String,
    keypair: Option<PathBuf>,
    handoff: Option<PathBuf>,
    alt: Option<String>,
    tips: Vec<String>,
    via_router: bool,
    send: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let rpc = RpcClient::new(rpc_url);
    let accounts = load_accounts(&rpc, keypair.as_deref(), handoff.as_deref(), &tips).await?;
    let built = build_signed(
        &rpc,
        &accounts,
        Pubkey::from_str(&target)?,
        amount,
        level.into(),
        alt.as_deref(),
        via_router,
    )
    .await?;
    let (transaction, size, decoys) = built;
    let signature = match simulate_and_send(&rpc, &transaction, SendOptions { broadcast: send })
        .await
    {
        Ok(signature) => signature,
        Err(error) if alt.is_some() => {
            eprintln!("ALT transaction failed ({error}); retrying without ALT");
            let (transaction, fallback_size, fallback_decoys) = build_signed(
                &rpc,
                &accounts,
                Pubkey::from_str(&target)?,
                amount,
                level.into(),
                None,
                via_router,
            )
            .await?;
            let signature =
                simulate_and_send(&rpc, &transaction, SendOptions { broadcast: send }).await?;
            println!(
                "Non-ALT fallback: {fallback_size}/{MAX_TX_PAYLOAD_BYTES} bytes, {fallback_decoys} decoys"
            );
            signature
        }
        Err(error) => return Err(Box::new(error)),
    };
    println!("RPC simulation succeeded; final transaction: {size}/{MAX_TX_PAYLOAD_BYTES} bytes, {decoys} decoys");
    match signature {
        Some(signature) => println!("Broadcast signature: {signature}"),
        None => println!("Broadcast skipped (pass --send to submit)"),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    match Cli::parse().command {
        Commands::Cook {
            sponsor_keypair,
            out_dir,
            rpc_url,
            cluster,
            sinks,
            fee_payer_lamports,
            sink_lamports,
            dry_run,
        } => {
            let sponsor = read_keypair_file(&sponsor_keypair)?;
            let rpc = RpcClient::new(rpc_url.clone());
            verify_rpc_cluster(&rpc, &cluster, &rpc_url).await?;
            let cooker = Cooker::new_offline(sponsor.pubkey());
            let config = CookerConfig {
                cluster,
                n_sinks: sinks,
                fund_fee_payer_lamports: fee_payer_lamports,
                fund_sink_lamports: sink_lamports,
                min_fee_payer_lamports: CONSERVATIVE_FEE_AND_DECOY_BUDGET,
                min_sink_lamports: 0,
            };
            let (mut handoff, keypairs) = cooker.generate(&config)?;
            let pubkeys = keypairs
                .iter()
                .map(|(_, keypair)| keypair.pubkey())
                .collect::<Vec<_>>();
            handoff.warnings = Cooker::detect_reuse_warnings(&out_dir, &pubkeys);
            handoff.accounts = Cooker::write_keypair_dir(&out_dir, &keypairs, &handoff.accounts)?;
            if dry_run {
                let warning = "dry-run: handoff is not cast-ready until accounts are funded";
                eprintln!("{warning}");
                handoff.warnings.push(warning.to_owned());
            } else {
                cooker
                    .fund_accounts(&rpc, &sponsor, &handoff, &out_dir)
                    .await?;
            }
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let path = out_dir.join(format!("handoff-{timestamp}.json"));
            Cooker::write_handoff(&path, &handoff)?;
            println!(
                "{} cooker handoff written to {}",
                if dry_run { "Dry-run" } else { "Funded" },
                path.display()
            );
        }
        Commands::Assemble {
            level,
            payer,
            target,
            amount,
        } => run_assemble(level, payer, target, amount)?,
        Commands::Simulate {
            level,
            target,
            amount,
            rpc_url,
            keypair,
            handoff,
            alt,
            tip,
            via_router,
        } => {
            run_cast(
                target, amount, level, rpc_url, keypair, handoff, alt, tip, via_router, false,
            )
            .await?;
        }
        Commands::Cast {
            target,
            amount,
            level,
            rpc_url,
            keypair,
            handoff,
            alt,
            tip,
            via_router,
            send,
        } => {
            run_cast(
                target, amount, level, rpc_url, keypair, handoff, alt, tip, via_router, send,
            )
            .await?;
        }
        Commands::Campaign {
            target,
            amount,
            level,
            rpc_url,
            keypair,
            handoff,
            alt,
            tip,
            txs,
            isolate_intent,
            send,
            drain_to,
        } => {
            if drain_to.is_some() && !send {
                return Err("--drain-to requires --send".into());
            }
            let rpc = RpcClient::new(rpc_url);
            let accounts =
                load_accounts(&rpc, keypair.as_deref(), handoff.as_deref(), &tip).await?;
            let payer = accounts.payer.pubkey();
            let plan = CampaignPlanner::new(payer, level.into())
                .with_sinks(accounts.sinks.clone())
                .isolate_intent(isolate_intent)
                .decoy_tx_count(txs)
                .plan(vec![system_instruction::transfer(
                    &payer,
                    &Pubkey::from_str(&target)?,
                    amount,
                )])?;
            let tables = resolve_alt(&rpc, alt.as_deref()).await?;
            let blockhash: Hash = rpc.get_latest_blockhash().await?;
            let mut prepared = Vec::with_capacity(plan.txs.len());
            for planned in plan.txs {
                let fallback_manifest = planned.manifest.clone();
                let built = FuzzyBundleBuilder::build_manifest_bundle(
                    payer,
                    planned.manifest,
                    blockhash,
                    &tables,
                )?;
                let instructions = FuzzyBundleBuilder::assemble_instructions(&built.manifest);
                let spend = transfer_spend(&instructions, &payer);
                let fee = message_fee(&rpc, &built.message).await?;
                let transaction = sign_versioned_tx(built.message, &[&accounts.payer])?;
                prepared.push((
                    planned.kind,
                    transaction,
                    built.serialized_size,
                    spend,
                    fee,
                    fallback_manifest,
                ));
            }
            let real_reserve = prepared
                .iter()
                .find(|(kind, _, _, _, _, _)| *kind == PlannedTxKind::RealIntent)
                .map(|(_, _, _, spend, fee, _)| spend.saturating_add(*fee))
                .ok_or("campaign plan has no real intent")?;

            for (kind, transaction, mut size, spend, fee, fallback_manifest) in prepared {
                let balance = rpc.get_balance(&payer).await?;
                let required = if kind == PlannedTxKind::RealIntent {
                    real_reserve
                } else {
                    real_reserve.saturating_add(spend).saturating_add(fee)
                };
                if kind != PlannedTxKind::RealIntent
                    && !decoy_preserves_reserve(balance, real_reserve, spend, fee)
                {
                    eprintln!(
                        "Skipping decoy transaction: balance {balance} would breach real-intent reserve {real_reserve}"
                    );
                    continue;
                }
                if balance < required {
                    return Err(Box::new(SupersonicError::Underfunded { balance, required })
                        as Box<dyn std::error::Error>);
                }
                let mut result =
                    simulate_and_send(&rpc, &transaction, SendOptions { broadcast: send }).await;
                if result.is_err() && !tables.is_empty() {
                    eprintln!("ALT campaign transaction failed; retrying without ALT");
                    let fallback = FuzzyBundleBuilder::build_manifest_bundle(
                        payer,
                        fallback_manifest,
                        rpc.get_latest_blockhash().await?,
                        &[],
                    )?;
                    size = fallback.serialized_size;
                    let transaction = sign_versioned_tx(fallback.message, &[&accounts.payer])?;
                    result = simulate_and_send(&rpc, &transaction, SendOptions { broadcast: send })
                        .await;
                }
                match (kind, result) {
                    (PlannedTxKind::RealIntent, Err(error)) => {
                        return Err(Box::new(error) as Box<dyn std::error::Error>)
                    }
                    (_, Err(error)) => eprintln!("Best-effort decoy transaction failed: {error}"),
                    (kind, Ok(signature)) => {
                        println!(
                            "{kind:?}: simulation succeeded, {size}/{MAX_TX_PAYLOAD_BYTES} bytes ({signature:?})"
                        )
                    }
                }
            }
            if let Some(destination) = drain_to {
                let (handoff, directory) = accounts
                    .handoff
                    .as_ref()
                    .ok_or("--drain-to requires --handoff")?;
                let sponsor = Pubkey::from_str(&handoff.sponsor_pubkey)?;
                Cooker::new_offline(sponsor)
                    .drain(
                        &rpc,
                        handoff,
                        directory,
                        &Pubkey::from_str(&destination)?,
                    )
                    .await
                    .map_err(|error| {
                        format!(
                            "campaign real intent succeeded, but post-campaign drain failed: {error}"
                        )
                    })?;
                println!("Post-campaign drain completed");
            }
        }
        Commands::Info => {
            println!(
                "supersonic-tx provides behavioral obscurity, not anonymity or fund concealment."
            );
            println!("RPC simulation and signed send are available; broadcast requires --send.");
            println!("Router noise is opt-in and requires an executable deployment check.");
            println!(
                "ALT accounts are fetched from RPC; unavailable ALTs fall back to non-ALT V0."
            );
            println!("Limits: sponsor tracing, timing correlation, human review, and router filtering remain.");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: &str = "4vMGoEDFfVJjF9y85sSvh4WwP76d9B54tE86b8xXN6T";

    #[test]
    fn invalid_level_is_rejected() {
        assert!(Cli::try_parse_from([
            "supersonic-tx",
            "cast",
            "--target",
            TARGET,
            "--keypair",
            "payer.json",
            "--level",
            "paranoidd",
        ])
        .is_err());
    }

    #[test]
    fn campaign_defaults_to_isolated_and_no_send() {
        let cli = Cli::try_parse_from([
            "supersonic-tx",
            "campaign",
            "--target",
            TARGET,
            "--handoff",
            "handoff.json",
        ])
        .unwrap();
        match cli.command {
            Commands::Campaign {
                isolate_intent,
                send,
                drain_to,
                ..
            } => {
                assert!(isolate_intent);
                assert!(!send);
                assert!(drain_to.is_none());
            }
            _ => panic!("expected campaign"),
        }
    }

    #[test]
    fn campaign_skips_decoy_at_real_intent_reserve_boundary() {
        assert!(!decoy_preserves_reserve(110, 100, 10, 1));
        assert!(decoy_preserves_reserve(111, 100, 10, 1));
    }

    #[test]
    fn assemble_requires_no_signer_or_target() {
        assert!(Cli::try_parse_from(["supersonic-tx", "assemble"]).is_ok());
    }

    #[test]
    fn simulate_requires_real_target_and_signer_input_flags_parse() {
        assert!(Cli::try_parse_from([
            "supersonic-tx",
            "simulate",
            "--target",
            TARGET,
            "--keypair",
            "payer.json",
        ])
        .is_ok());
    }

    #[test]
    fn test_cli_parse_cook() {
        let args = vec![
            "supersonic-tx",
            "cook",
            "--sponsor-keypair",
            "/tmp/sponsor.json",
            "--out-dir",
            "/tmp/cook",
            "--rpc-url",
            "https://api.devnet.solana.com",
            "--sinks",
            "2",
        ];
        let cli = Cli::try_parse_from(args).expect("cook parse");
        match cli.command {
            Commands::Cook {
                sponsor_keypair,
                out_dir,
                sinks,
                ..
            } => {
                assert_eq!(sponsor_keypair, PathBuf::from("/tmp/sponsor.json"));
                assert_eq!(out_dir, PathBuf::from("/tmp/cook"));
                assert_eq!(sinks, 2);
            }
            _ => panic!("expected Cook"),
        }
    }
}

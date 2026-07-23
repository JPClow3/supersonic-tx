use account_cooker::{CookedRole, Cooker, CookerConfig};
use clap::{Parser, Subcommand, ValueEnum};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use solana_sdk::system_instruction;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use supersonic_tx_core::{ObfuscationLevel, SupersonicError, MAX_TX_PAYLOAD_BYTES};
use supersonic_tx_sdk::{
    sign_versioned_tx, simulate_and_send, verify_executable_program, AltResolver, CampaignPlanner,
    DecoySink, FuzzyBundleBuilder, PlannedTxKind, SendOptions,
};

const DEFAULT_RPC_URL: &str = "https://api.devnet.solana.com";
const CONSERVATIVE_FEE_AND_DECOY_BUDGET: u64 = 255_000;

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
    },
    Info,
}

struct LoadedAccounts {
    payer: Keypair,
    sinks: Vec<DecoySink>,
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

    let (payer, mut sinks) = if let Some(path) = handoff_path {
        let handoff = Cooker::load_handoff(path)?;
        Cooker::assert_funded_for_cast(&handoff, CONSERVATIVE_FEE_AND_DECOY_BUDGET)?;
        let directory = path.parent().unwrap_or_else(|| Path::new("."));
        let keypairs = Cooker::resolve_keypairs(&handoff, directory)?;
        let mut payer = None;
        let mut sinks = Vec::new();
        for (account, keypair) in handoff.accounts.iter().zip(keypairs.into_iter()) {
            match account.role {
                CookedRole::FeePayer => payer = Some(keypair),
                CookedRole::DecoySink => {
                    let destination = Pubkey::from_str(&account.pubkey)?;
                    sinks.push(DecoySink::validate_on_chain(rpc, destination).await?);
                }
                CookedRole::DrainTarget => {}
            }
        }
        (payer.ok_or("handoff has no fee payer keypair")?, sinks)
    } else {
        (
            read_keypair_file(keypair_path.expect("checked exclusive input"))?,
            Vec::new(),
        )
    };

    for tip in tips {
        sinks.push(DecoySink::validate_on_chain(rpc, Pubkey::from_str(tip)?).await?);
    }
    Ok(LoadedAccounts { payer, sinks })
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
    let mut builder = FuzzyBundleBuilder::new(payer, level)
        .add_target_instruction(system_instruction::transfer(&payer, &target, amount));
    builder = if accounts.sinks.is_empty() {
        eprintln!("Transfer noise disabled: no RPC-validated system-wallet sinks were supplied");
        builder.without_transfer_noise()
    } else {
        builder.with_sinks(accounts.sinks.clone())?
    };
    if via_router {
        let router = supersonic_tx_core::program_id();
        verify_executable_program(rpc, &router).await?;
        builder = builder.with_router_noise(router);
    }

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
        return Err(Box::new(SupersonicError::Underfunded { balance, required }));
    }
    let decoy_count = built.manifest.decoy_instructions.len();
    let transaction = sign_versioned_tx(built.message, &[&accounts.payer])?;
    Ok((transaction, built.serialized_size, decoy_count))
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
    let (transaction, size, decoys) = build_signed(
        &rpc,
        &accounts,
        Pubkey::from_str(&target)?,
        amount,
        level.into(),
        alt.as_deref(),
        via_router,
    )
    .await?;
    let signature = simulate_and_send(&rpc, &transaction, SendOptions { broadcast: send }).await?;
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
            Cooker::write_keypair_dir(&out_dir, &keypairs)?;
            if dry_run {
                let warning = "dry-run: handoff is not cast-ready until accounts are funded";
                eprintln!("{warning}");
                handoff.warnings.push(warning.to_owned());
            } else {
                let rpc = RpcClient::new(rpc_url);
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
        } => {
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
            for planned in plan.txs {
                let result = async {
                    let blockhash: Hash = rpc
                        .get_latest_blockhash()
                        .await
                        .map_err(|error| SupersonicError::RpcError(error.to_string()))?;
                    let message: VersionedMessage = FuzzyBundleBuilder::compile_v0_message(
                        &payer,
                        &planned.instructions,
                        &tables,
                        blockhash,
                    )?;
                    let size = FuzzyBundleBuilder::estimate_tx_size(&message)?;
                    if size > MAX_TX_PAYLOAD_BYTES {
                        return Err(SupersonicError::TransactionSizeExceeded(size));
                    }
                    let transaction = sign_versioned_tx(message, &[&accounts.payer])?;
                    simulate_and_send(&rpc, &transaction, SendOptions { broadcast: send }).await
                }
                .await;
                match (planned.kind, result) {
                    (PlannedTxKind::RealIntent, Err(error)) => return Err(Box::new(error)),
                    (_, Err(error)) => eprintln!("Best-effort decoy transaction failed: {error}"),
                    (kind, Ok(signature)) => {
                        println!("{kind:?}: simulation succeeded ({signature:?})")
                    }
                }
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
                ..
            } => {
                assert!(isolate_intent);
                assert!(!send);
            }
            _ => panic!("expected campaign"),
        }
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
                assert_eq!(sponsor_keypair, "/tmp/sponsor.json");
                assert_eq!(out_dir, "/tmp/cook");
                assert_eq!(sinks, 2);
            }
            _ => panic!("expected Cook"),
        }
    }
}

use std::{path::PathBuf, str::FromStr};

use clap::{
    builder::{StringValueParser, TypedValueParser, ValueParser},
    Parser,
};
use consts::{
    CHECK_MAX_RETRY, DEFAULT_COMPUTE_UNIT_LIMIT, DEFAULT_COMPUTE_UNIT_PRICE, TRANSFER_MAX_RETRY,
};
use flexi_logger::Logger;
use sanctum_solana_cli_utils::ConfigWrapper;

use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::{
    data::WalletList,
    errors::{Error, Result},
    utils::get_token_mint_info,
};

mod consts;
mod data;
pub mod errors;
mod utils;

#[derive(Parser, Debug)]
#[command(version, about = "sanctum airdrop sender program")]
struct Args {
    #[arg(
        long,
        short,
        help = "Path to solana CLI config. Defaults to solana cli default if not provided",
        default_value = "",
        value_parser = ValueParser::new(ConfigWrapper::parse_from_path),
    )]
    pub config: ConfigWrapper,

    #[arg(
        long,
        short,
        help = "Path to wallet_list csv file in the format of \"wallet_pubkey,amount_to_airdrop\""
    )]
    pub wallet_list_path: PathBuf,

    #[arg(
        long,
        short,
        help = "",
        value_parser = StringValueParser::new().try_map(|s| Pubkey::from_str(&s)),
    )]
    pub token_mint_pubkey: Pubkey,

    #[arg(
        long,
        short,
        help = "Compute unit limit",
        default_value_t = DEFAULT_COMPUTE_UNIT_LIMIT,
    )]
    pub compute_unit_limit: u32,

    #[arg(
        long,
        short,
        help = "Compute unit price in micro lamports",
        default_value_t = DEFAULT_COMPUTE_UNIT_PRICE,
    )]
    pub compute_unit_price: u64,

    #[arg(
        long,
        short,
        help = "Path to payer keypair who holds the token to be airdropped"
    )]
    pub payer_path: PathBuf,

    #[arg(long, short)]
    pub dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    Logger::try_with_str("error, airdropper=debug")
        .unwrap()
        .start()
        .unwrap();

    let args = Args::parse();
    let rpc_client = args.config.rpc_client();
    // NOTE: don't use args.config.signer() for now
    let payer = read_keypair_file(
        args.payer_path
            .to_str()
            .expect("Could not convert payer_path to str"),
    )
    .map_err(|_e| Error::KeyPairError)?;
    let (token_program_id, token_decimals) =
        get_token_mint_info(&rpc_client, &args.token_mint_pubkey)?;
    let source_ata = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &args.token_mint_pubkey,
        &token_program_id,
    );

    // TODO: check if source ata has enough balance

    log::info!("Token mint pubkey: {:?}", args.token_mint_pubkey);
    log::info!("Token program id: {token_program_id:?}");
    log::info!("Token decimals: {token_decimals}");
    log::info!("Source ATA: {source_ata:?}");

    let mut wallet_list = WalletList::parse_list_from_path(args.wallet_list_path)?;
    let wallet_count = wallet_list.0.len();
    log::info!("Wallet count: {wallet_count}");

    for i in 1..=CHECK_MAX_RETRY {
        log::info!("Checking the airdrop qualification ...");
        wallet_list.check_unprocessed(
            &rpc_client,
            &args.token_mint_pubkey,
            &token_program_id,
            token_decimals,
        );

        // TODO: sync
        // log::info!("Syncing data ...");
        // wallet_list.save_to_path();

        let failed_count = wallet_list.count_failed();
        if failed_count == 0 {
            break;
        }

        log::info!("Failed to check ({failed_count} / {wallet_count})");
        if i != CHECK_MAX_RETRY {
            wallet_list.set_failed_to_unprocessed();
        } else {
            wallet_list.set_failed_to_excluded();
        }
        // TODO: sync
        // log::info!("Syncing data ...");
        // wallet_list.save_to_path();
    }

    let qualified_wallet_count = wallet_list.count_qualified();
    for i in 1..=TRANSFER_MAX_RETRY {
        log::info!("Transferring the airdrop ...",);
        wallet_list.transfer_airdrop(
            &rpc_client,
            &args.token_mint_pubkey,
            &token_program_id,
            token_decimals,
            &source_ata,
            &payer,
            args.compute_unit_limit,
            args.compute_unit_price,
            args.dry_run,
        );

        // TODO: check to resolve false positives in entries with Failed status

        // TODO: sync
        // log::info!("Syncing data ...");
        // wallet_list.save_to_path();

        let failed_count = wallet_list.count_failed();
        if failed_count == 0 {
            break;
        }

        log::info!("Failed to transfer ({failed_count} / {qualified_wallet_count})");
        if i != TRANSFER_MAX_RETRY {
            wallet_list.set_failed_to_qualified();
        }
        // TODO: sync
        // log::info!("Syncing data ...");
        // wallet_list.save_to_path();
    }

    log::info!("DONE");

    Ok(())
}

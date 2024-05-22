use std::{path::PathBuf, str::FromStr, thread, time::Duration};

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
use subcmd::Subcmd;

use crate::{
    consts::{CONFIRM_TX_MAX_RETRY, CONFIRM_TX_SLEEP_SEC},
    data::WalletList,
    errors::{Error, Result},
    utils::get_token_mint_info,
};

mod consts;
mod data;
pub mod errors;
mod subcmd;
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
        short = 'l',
        help = "Compute unit limit",
        default_value_t = DEFAULT_COMPUTE_UNIT_LIMIT,
    )]
    pub compute_unit_limit: u32,

    #[arg(
        long,
        short = 'p',
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

    // #[arg(
    //     long,
    //     short,
    //     help = "Path to the status list file to be saved during the execution"
    // )]
    // pub status_list_path: PathBuf,
    #[arg(long, short)]
    pub dry_run: bool,

    #[arg(
        long,
        short,
        help = "After sending transaction, wait for confirmation before proceeding"
    )]
    pub should_confirm: bool,

    #[command(subcommand)]
    pub subcmd: Subcmd,
}

fn main() -> Result<()> {
    Logger::try_with_str("error, airdropper=debug")
        .unwrap()
        .start()
        .unwrap();

    let args = Args::parse();
    subcmd::Subcmd::run(args)

    // let rpc_client = args.config.rpc_client();
    // // NOTE: don't use args.config.signer() for now
    // let payer = read_keypair_file(
    //     args.payer_path
    //         .to_str()
    //         .expect("Could not convert payer_path to str"),
    // )
    // .map_err(|_e| Error::KeyPairError)?;
    // let (token_program_id, token_decimals) =
    //     get_token_mint_info(&rpc_client, &args.token_mint_pubkey)?;
    // let source_ata = get_associated_token_address_with_program_id(
    //     &payer.pubkey(),
    //     &args.token_mint_pubkey,
    //     &token_program_id,
    // );

    // // TODO: check if source ata has enough balance

    // log::info!("Token mint pubkey: {:?}", args.token_mint_pubkey);
    // log::info!("Token program id: {token_program_id:?}");
    // log::info!("Token decimals: {token_decimals}");
    // log::info!("Source ATA: {source_ata:?}");

    // let mut wallet_list = WalletList::parse_list_from_path(args.wallet_list_path)?;
    // let wallet_count = wallet_list.0.len();
    // log::info!("Wallet count: {wallet_count}");

    // // for resuming execution, try confirm unconfirmed and set unconfirmed to failed
    // if wallet_list.count_unconfirmed() != 0 {
    //     log::info!("Attempting to confirm unconfirmed trnasactions ...");
    //     let n_total_unconfirmed = wallet_list.confirm(&rpc_client);
    //     log::info!("Resetting {n_total_unconfirmed} to failed");
    //     wallet_list.set_unconfirmed_to_failed();
    // }
    // wallet_list.set_failed_to_unprocessed();

    // for check_trial_count in 1..=CHECK_MAX_RETRY {
    //     log::info!("Checking the airdrop qualification ...");
    //     wallet_list.check_unprocessed(
    //         &rpc_client,
    //         &args.token_mint_pubkey,
    //         &token_program_id,
    //         token_decimals,
    //     );

    //     wallet_list
    //         .save_to_path(&args.status_list_path)
    //         .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));

    //     let failed_count = wallet_list.count_failed();
    //     if failed_count == 0 {
    //         log::info!("Finished checking all wallets");
    //         break;
    //     }

    //     log::info!("Failed to check ({failed_count} / {wallet_count})");
    //     if check_trial_count != CHECK_MAX_RETRY {
    //         wallet_list.set_failed_to_unprocessed();
    //     } else {
    //         log::info!("");
    //         wallet_list.set_failed_to_excluded();
    //     }

    //     wallet_list
    //         .save_to_path(&args.status_list_path)
    //         .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));
    // }

    // let qualified_wallet_count = wallet_list.count_qualified();
    // log::info!("Found {qualified_wallet_count} qualified wallets");
    // for transfer_trial_count in 1..=TRANSFER_MAX_RETRY {
    //     log::info!("Transferring the airdrop ...",);
    //     wallet_list.transfer_airdrop(
    //         &rpc_client,
    //         &args.token_mint_pubkey,
    //         &token_program_id,
    //         token_decimals,
    //         &source_ata,
    //         &payer,
    //         args.compute_unit_limit,
    //         args.compute_unit_price,
    //         args.dry_run,
    //     );

    //     wallet_list
    //         .save_to_path(&args.status_list_path)
    //         .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));

    //     for _confirm_trial_count in 1..=CONFIRM_TX_MAX_RETRY {
    //         let n_total_unconfirmed = wallet_list.confirm(&rpc_client);
    //         if n_total_unconfirmed == 0 {
    //             break;
    //         }
    //         log::debug!("Retrying in {CONFIRM_TX_SLEEP_SEC} sec");
    //         thread::sleep(Duration::from_secs(CONFIRM_TX_SLEEP_SEC));
    //     }

    //     // NOTE: if a tx is not settled at this point, consider it as failed
    //     wallet_list.set_unconfirmed_to_failed();

    //     wallet_list
    //         .save_to_path(&args.status_list_path)
    //         .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));

    //     let failed_count = wallet_list.count_failed();
    //     if failed_count == 0 {
    //         break;
    //     }

    //     log::info!("Failed to transfer ({failed_count} / {qualified_wallet_count})");

    //     if transfer_trial_count != TRANSFER_MAX_RETRY {
    //         wallet_list.set_failed_to_qualified();
    //     }
    // }

    // log::info!("DONE");

    // Ok(())
}

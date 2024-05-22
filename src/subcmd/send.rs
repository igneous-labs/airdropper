use std::{path::PathBuf, str::FromStr};

use clap::{
    builder::{StringValueParser, TypedValueParser},
    Args,
};
use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::{
    consts::{DEFAULT_COMPUTE_UNIT_LIMIT, DEFAULT_COMPUTE_UNIT_PRICE},
    data::WalletList,
    errors::{Error, Result},
    subcmd::Subcmd,
    utils::{
        add_to_filename, create_backup_if_file_exists, get_token_mint_info, prompt_confirmation,
    },
};

#[derive(Args, Debug)]
#[command(long_about = "Send airdrop transactions")]
pub struct SendArgs {
    #[arg(
        long,
        short,
        help = "Mint pubkey of the token to be airdropped",
        value_parser = StringValueParser::new().try_map(|s| Pubkey::from_str(&s)),
    )]
    airdrop_token_mint_pubkey: Pubkey,

    #[arg(
        long,
        short,
        help = "Path to payer keypair who holds the token to be airdropped"
    )]
    payer_path: PathBuf,

    #[arg(
        long,
        short = 'l',
        help = "Compute unit limit",
        default_value_t = DEFAULT_COMPUTE_UNIT_LIMIT,
    )]
    compute_unit_limit: u32,

    #[arg(
        long,
        short = 'p',
        help = "Compute unit price in micro lamports",
        default_value_t = DEFAULT_COMPUTE_UNIT_PRICE,
    )]
    compute_unit_price: u64,

    #[arg(
        long,
        short,
        help = "After sending transaction, wait for confirmation before proceeding"
    )]
    should_confirm: bool,
}

impl SendArgs {
    pub fn run(args: crate::Args) -> Result<()> {
        let Self {
            airdrop_token_mint_pubkey,
            payer_path,
            compute_unit_limit,
            compute_unit_price,
            should_confirm,
        } = match args.subcmd {
            Subcmd::Send(a) => a,
            _ => unreachable!(),
        };

        let rpc_client = args.config.rpc_client();
        let (token_program_id, token_decimals) =
            get_token_mint_info(&rpc_client, &airdrop_token_mint_pubkey)?;
        let payer = read_keypair_file(
            payer_path
                .to_str()
                .expect("Could not convert payer_path to str"),
        )
        .map_err(|_e| Error::KeyPairError)?;
        let source_ata = get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &airdrop_token_mint_pubkey,
            &token_program_id,
        );

        // Note: assume that the either check stage or confirmed stage ran beforehand
        let check_stage_save_path = add_to_filename(&args.wallet_list_path, "checked");
        let confirm_stage_save_path = add_to_filename(&args.wallet_list_path, "confirmed");
        let current_stage_save_path = add_to_filename(&args.wallet_list_path, "sent");

        let mut wallet_list = if confirm_stage_save_path.try_exists()? {
            log::info!("Detected saved confirm stage, retrying confirmation ...");
            let mut wallet_list = WalletList::parse_list_from_path(&confirm_stage_save_path)?;
            // NOTE: make sure the confirm stage file from last send attempt is cleared (saved as backup) for the next confirm stage
            if !args.dry_run {
                create_backup_if_file_exists(&confirm_stage_save_path)?;
            }
            if wallet_list.count_unconfirmed() != 0 {
                log::info!("Attempting to confirm unconfirmed trnasactions ...");
                let n_total_unconfirmed = wallet_list.confirm(&rpc_client);
                log::info!("Resetting {n_total_unconfirmed} to failed");
                wallet_list.set_unconfirmed_to_failed();
            }
            log::info!("Resetting failed to qualified");
            wallet_list.set_failed_to_qualified();
            wallet_list
        } else if check_stage_save_path.try_exists()? {
            if current_stage_save_path.try_exists()? {
                log::warn!("Could not find saved confirm stage for the last send stage (possibly running send stage twice?)");
            }
            WalletList::parse_list_from_path(&check_stage_save_path)?
        } else {
            return Err(Error::StageNotReady);
        };

        if !args.dry_run && !prompt_confirmation("About to send txs. Should we proceed?") {
            log::info!("Terminating");
            return Ok(());
        }
        log::info!("Transferring the airdrop ...",);
        wallet_list.transfer_airdrop(
            &rpc_client,
            &airdrop_token_mint_pubkey,
            &token_program_id,
            token_decimals,
            &source_ata,
            &payer,
            compute_unit_limit,
            compute_unit_price,
            args.dry_run,
            should_confirm,
        );

        if !args.dry_run {
            wallet_list
                .save_to_path(&current_stage_save_path)
                .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));
        }

        Ok(())
    }
}

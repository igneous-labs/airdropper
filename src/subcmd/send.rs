use solana_sdk::{signature::read_keypair_file, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::{
    data::WalletList,
    errors::{Error, Result},
    utils::{
        add_to_filename, create_backup_if_file_exists, get_token_mint_info, prompt_confirmation,
    },
};

pub fn run(args: crate::Args) -> Result<()> {
    let rpc_client = args.config.rpc_client();
    let (token_program_id, token_decimals) =
        get_token_mint_info(&rpc_client, &args.token_mint_pubkey)?;
    let payer = read_keypair_file(
        args.payer_path
            .to_str()
            .expect("Could not convert payer_path to str"),
    )
    .map_err(|_e| Error::KeyPairError)?;
    let source_ata = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &args.token_mint_pubkey,
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

    if !prompt_confirmation("About to send txs. Should we proceed?") {
        log::info!("Terminating");
        return Ok(());
    }
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
        args.should_confirm,
    );

    if !args.dry_run {
        wallet_list
            .save_to_path(&current_stage_save_path)
            .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));
    }

    Ok(())
}

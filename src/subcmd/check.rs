use crate::{
    consts::CHECK_MAX_RETRY,
    data::WalletList,
    errors::Result,
    utils::{add_to_filename, get_token_mint_info},
};

pub fn run(args: crate::Args) -> Result<()> {
    let rpc_client = args.config.rpc_client();
    let (token_program_id, token_decimals) =
        get_token_mint_info(&rpc_client, &args.token_mint_pubkey)?;

    let mut wallet_list = WalletList::parse_list_from_path(&args.wallet_list_path)?;
    let wallet_count = wallet_list.0.len();

    log::info!("Wallet count: {wallet_count}");
    let stage_save_path = add_to_filename(&args.wallet_list_path, "checked");

    for check_trial_count in 1..=CHECK_MAX_RETRY {
        log::info!("Checking the airdrop qualification ...");
        wallet_list.check_unprocessed(
            &rpc_client,
            &args.token_mint_pubkey,
            &token_program_id,
            token_decimals,
        );

        wallet_list
            .save_to_path(&stage_save_path)
            .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));

        let failed_count = wallet_list.count_failed();
        if failed_count == 0 {
            log::info!("Finished checking all wallets");
            break;
        }

        log::info!("Failed to check ({failed_count} / {wallet_count})");
        if check_trial_count != CHECK_MAX_RETRY {
            wallet_list.set_failed_to_unprocessed();
        } else {
            log::info!("");
            wallet_list.set_failed_to_excluded();
        }

        if !args.dry_run {
            wallet_list
                .save_to_path(&stage_save_path)
                .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));
        }
    }

    let qualified_wallet_count = wallet_list.count_qualified();
    log::info!("Found {qualified_wallet_count} qualified wallets");

    Ok(())
}

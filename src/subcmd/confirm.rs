use std::path::PathBuf;

use clap::Args;

use crate::{
    data::{CsvListSerde, WalletList},
    errors::{Error, Result},
    subcmd::Subcmd,
    utils::add_to_filename,
};

#[derive(Args, Debug)]
#[command(long_about = "Given a sent wallet list, confirm unconfirmed transactions")]
pub struct ConfirmArgs {
    #[arg(long, short, help = "Path to wallet list csv file")]
    pub wallet_list_path: PathBuf,
}

impl ConfirmArgs {
    pub fn run(args: crate::Args) -> Result<()> {
        let Self { wallet_list_path } = match args.subcmd {
            Subcmd::Confirm(a) => a,
            _ => unreachable!(),
        };
        let rpc_client = args.config.rpc_client();

        let send_stage_save_path = add_to_filename(&wallet_list_path, "sent");
        let confirm_stage_save_path = add_to_filename(&wallet_list_path, "confirmed");

        let base_stage_save_path = if confirm_stage_save_path.try_exists()? {
            log::info!("Detected saved confirm stage, retrying confirmation ...");
            confirm_stage_save_path
        } else if send_stage_save_path.try_exists()? {
            send_stage_save_path
        } else {
            return Err(Error::StageNotReady);
        };

        let mut wallet_list = WalletList::parse_list_from_path(&base_stage_save_path)?;
        let total_unconfirmed_count = wallet_list.get_unconfirmed_sigs().len();
        if total_unconfirmed_count == 0 {
            log::info!("No unconfirmed txs, terminating");
            return Ok(());
        }
        log::info!(
            "Found {} txs to confirm, confirming ...",
            total_unconfirmed_count,
        );
        let current_unconfirmed_count = wallet_list.confirm(&rpc_client);
        log::info!(
            "Confirmed: {}; Unconfirmed: {}",
            total_unconfirmed_count - current_unconfirmed_count,
            current_unconfirmed_count
        );
        let stage_save_path = add_to_filename(&wallet_list_path, "confirmed");

        if !args.dry_run {
            wallet_list
                .save_to_path(&stage_save_path)
                .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));
        }

        Ok(())
    }
}

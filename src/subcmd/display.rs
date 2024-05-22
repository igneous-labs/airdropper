use clap::Args;

use crate::{data::WalletList, errors::Result};

#[derive(Args, Debug)]
#[command(long_about = "Display wallet list content")]
pub struct DisplayArgs;

impl DisplayArgs {
    pub fn run(args: crate::Args) -> Result<()> {
        let wallet_list = WalletList::parse_list_from_path(&args.wallet_list_path)?;

        let counts = wallet_list.count_each_status();
        log::info!("{counts:#?}");

        Ok(())
    }
}

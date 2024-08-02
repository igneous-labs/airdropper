use std::path::PathBuf;

use clap::Args;

use crate::{
    data::{CsvListSerde, WalletList},
    errors::Result,
    subcmd::Subcmd,
};

#[derive(Args, Debug)]
#[command(long_about = "Display wallet list content")]
pub struct DisplayArgs {
    #[arg(
        long,
        short,
        help = "Path to wallet_list csv file in the format of \"wallet_pubkey,amount_to_airdrop\""
    )]
    pub wallet_list_path: PathBuf,
}

impl DisplayArgs {
    pub fn run(args: crate::Args) -> Result<()> {
        let Self { wallet_list_path } = match args.subcmd {
            Subcmd::Display(a) => a,
            _ => unreachable!(),
        };
        let wallet_list = WalletList::parse_list_from_path(&wallet_list_path)?;

        let counts = wallet_list.count_each_status();
        log::info!("{counts:#?}");

        Ok(())
    }
}

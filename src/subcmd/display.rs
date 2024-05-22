use crate::{data::WalletList, errors::Result};

pub fn run(args: crate::Args) -> Result<()> {
    let wallet_list = WalletList::parse_list_from_path(&args.wallet_list_path)?;

    let counts = wallet_list.count_each_status();
    log::info!("{counts:#?}");

    Ok(())
}

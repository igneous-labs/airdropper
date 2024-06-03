use std::path::PathBuf;

use clap::Args;

use crate::{
    data::{CsvListSerde, Snapshot, SnapshotEntry, WalletList, WalletListEntry},
    errors::Result,
    subcmd::Subcmd,
};

#[derive(Args, Debug)]
#[command(long_about = "Given a token snapshot and a airdrop amount, generate a wallet list")]
pub struct WalletListArgs {
    #[arg(long, short, help = "Path to wallet list csv file")]
    pub wallet_list_path: PathBuf,

    #[arg(long, short, help = "The total amount (in token atomic) to airdrop")]
    amount_to_airdrop: u64,

    #[arg(long, short, help = "Path to token snapshot csv file")]
    snapshot_path: PathBuf,
}

impl WalletListArgs {
    pub fn run(args: crate::Args) -> Result<()> {
        let Self {
            wallet_list_path,
            amount_to_airdrop,
            snapshot_path,
        } = match args.subcmd {
            Subcmd::WalletList(a) => a,
            _ => unreachable!(),
        };

        let snapshot = Snapshot::parse_list_from_path(&snapshot_path)?;

        let total_amount: u64 = snapshot
            .0
            .iter()
            .map(
                |SnapshotEntry {
                     token_balance_atomic,
                     ..
                 }| token_balance_atomic,
            )
            .sum();
        let mut wallet_list = WalletList(
            snapshot
                .0
                .into_iter()
                .filter_map(
                    |SnapshotEntry {
                         wallet_pubkey,
                         token_balance_atomic,
                     }| {
                        let amount_to_airdrop =
                            (token_balance_atomic as u128 * amount_to_airdrop as u128
                                / total_amount as u128) as u64;
                        if amount_to_airdrop != 0 {
                            Some(WalletListEntry {
                                wallet_pubkey,
                                amount_to_airdrop,
                                ..Default::default()
                            })
                        } else {
                            None
                        }
                    },
                )
                .collect::<Vec<_>>(),
        );
        log::info!("Total wallet list count: {}", wallet_list.0.len());

        let total_amount_from_wallet_list = wallet_list
            .0
            .iter()
            .map(|entry| entry.amount_to_airdrop)
            .sum::<u64>();
        log::info!(
            "Total amount in wallet list: {}",
            total_amount_from_wallet_list
        );
        assert!(total_amount_from_wallet_list <= total_amount);

        if !args.dry_run {
            wallet_list
                .save_to_path(&wallet_list_path)
                .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));
        }

        Ok(())
    }
}

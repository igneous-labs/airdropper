use std::path::PathBuf;

use clap::{builder::ValueParser, Parser};
use flexi_logger::Logger;
use sanctum_solana_cli_utils::ConfigWrapper;

use crate::{errors::Result, subcmd::Subcmd};

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

    #[arg(long, short)]
    pub dry_run: bool,

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
}

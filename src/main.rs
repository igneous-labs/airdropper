use clap::{builder::ValueParser, Parser};
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
        help = "dry run (note: if set, does not save any files nor send any transactions)"
    )]
    pub dry_run: bool,

    #[command(subcommand)]
    pub subcmd: Subcmd,
}

fn main() -> Result<()> {
    flexi_logger::Logger::try_with_env_or_str("error, airdropper=debug")
        .unwrap()
        .append()
        .log_to_file(flexi_logger::FileSpec::default().suppress_timestamp()) // write logs to file
        .duplicate_to_stderr(flexi_logger::Duplicate::All)
        .start()
        .unwrap();

    let args = Args::parse();
    subcmd::Subcmd::run(args)
}

use clap::Subcommand;

use crate::errors::Result;

mod check;
mod confirm;
mod display;
mod send;

#[derive(Debug, Subcommand)]
pub enum Subcmd {
    Check,
    Send,
    Confirm,
    Display,
}

impl Subcmd {
    pub fn run(args: crate::Args) -> Result<()> {
        match args.subcmd {
            Self::Check => check::run(args),
            Self::Send => send::run(args),
            Self::Confirm => confirm::run(args),
            Self::Display => display::run(args),
        }
    }
}

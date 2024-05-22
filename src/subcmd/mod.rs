use clap::Subcommand;

use crate::errors::Result;

use self::{
    check::CheckArgs, confirm::ConfirmArgs, display::DisplayArgs, send::SendArgs,
    snapshot::SnapshotArgs,
};

mod check;
mod confirm;
mod display;
mod send;
mod snapshot;

#[derive(Debug, Subcommand)]
pub enum Subcmd {
    Snapshot(SnapshotArgs),
    Check(CheckArgs),
    Send(SendArgs),
    Confirm(ConfirmArgs),
    Display(DisplayArgs),
}

impl Subcmd {
    pub fn run(args: crate::Args) -> Result<()> {
        match args.subcmd {
            Self::Snapshot(_) => SnapshotArgs::run(args),
            Self::Check(_) => CheckArgs::run(args),
            Self::Send(_) => SendArgs::run(args),
            Self::Confirm(_) => ConfirmArgs::run(args),
            Self::Display(_) => DisplayArgs::run(args),
        }
    }
}

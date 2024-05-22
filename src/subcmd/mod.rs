use clap::Subcommand;

use crate::errors::Result;

use self::{check::CheckArgs, send::SendArgs, snapshot::SnapshotArgs};

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
    Confirm,
    Display,
}

impl Subcmd {
    pub fn run(args: crate::Args) -> Result<()> {
        match args.subcmd {
            Self::Snapshot(_) => SnapshotArgs::run(args),
            Self::Check(_) => CheckArgs::run(args),
            Self::Send(_) => SendArgs::run(args),
            Self::Confirm => confirm::run(args),
            Self::Display => display::run(args),
        }
    }
}

use std::{path::PathBuf, str::FromStr};

use solana_program::pubkey::Pubkey;

use crate::errors::{Error, Result};

use super::{CsvEntrySer, CsvListSerde};

#[derive(Debug)]
pub struct Snapshot(pub Vec<SnapshotEntry>);

#[derive(Debug, serde::Deserialize, Clone)]
pub struct SnapshotEntryRaw {
    pub wallet_pubkey: String,
    pub token_balance_atomic: u64,
}

#[derive(Debug)]
pub struct SnapshotEntry {
    pub wallet_pubkey: Pubkey,
    pub token_balance_atomic: u64,
}

impl CsvEntrySer for SnapshotEntry {
    fn to_record(&self) -> Vec<String> {
        vec![
            self.wallet_pubkey.to_string(),
            self.token_balance_atomic.to_string(),
        ]
    }
}

impl TryFrom<SnapshotEntryRaw> for SnapshotEntry {
    type Error = Error;

    fn try_from(
        SnapshotEntryRaw {
            wallet_pubkey,
            token_balance_atomic,
        }: SnapshotEntryRaw,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(SnapshotEntry {
            wallet_pubkey: Pubkey::from_str(&wallet_pubkey)?,
            token_balance_atomic,
        })
    }
}

impl CsvListSerde for Snapshot {
    fn parse_list_from_path(path: &PathBuf) -> Result<Self> {
        log::info!("Parsing snapshot from {path:?} ...");
        let data = std::fs::read_to_string(path)?;
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(false)
            .from_reader(data.as_bytes());
        let list = rdr
            .deserialize()
            .collect::<std::result::Result<Vec<SnapshotEntryRaw>, _>>()?;
        println!("WTF1: {}", list.len());
        let mut list = list
            .into_iter()
            .map(SnapshotEntry::try_from)
            .collect::<std::result::Result<Vec<SnapshotEntry>, _>>()?;
        println!("WTF2: {}", list.len());
        list.sort_by(|a, b| a.wallet_pubkey.cmp(&b.wallet_pubkey));
        log::info!("Finished parsing snapshot");
        Ok(Self(list))
    }

    fn save_to_path(&mut self, path: &PathBuf) -> Result<()> {
        log::info!("Saving snapshot to {path:?} ...");
        let mut wtr = csv::Writer::from_path(path)?;
        self.0.sort_by(|a, b| a.wallet_pubkey.cmp(&b.wallet_pubkey));
        for entry in self.0.iter() {
            wtr.write_record(entry.to_record())?;
        }
        wtr.flush()?;
        log::info!("Finished saving status data");
        Ok(())
    }
}

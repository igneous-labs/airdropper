use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::PathBuf,
    str::FromStr,
};

use solana_client::rpc_client::{RpcClient, SerializableTransaction};
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_sdk::{signature::Signature, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::instruction::transfer_checked;

use crate::{
    consts::{ATA_GET_MULT_ACC_CHUNK_SIZE, TRANSFER_IXS_CHUNK_SIZE},
    errors::{Error, Result},
    utils::{check_atas, create_backup_if_file_exists, get_compute_budget_ixs, prep_tx},
};

use super::{CsvEntrySer, CsvListSerde};

// TODO: use serde with
#[derive(Debug, serde::Deserialize, Clone)]
struct WalletListEntryRaw {
    pub wallet_pubkey: String,
    pub amount_to_airdrop: u64,
    #[serde(default)]
    pub ata: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub status_inner: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub enum Status {
    #[default]
    Unprocessed,
    Disqualified,
    Qualified,
    Unconfirmed(Signature),
    Failed(String),
    Succeeded(Signature),
    Excluded(String),
}

impl Status {
    fn to_record(&self) -> (String, Option<String>) {
        match self {
            Self::Unprocessed => ("unprocessed".to_string(), None),
            Self::Disqualified => ("disqualified".to_string(), None),
            Self::Qualified => ("qualified".to_string(), None),
            Self::Unconfirmed(sig) => ("unconfirmed".to_string(), Some(sig.to_string())),
            Self::Failed(err) => ("failed".to_string(), Some(err.to_string())),
            Self::Succeeded(sig) => ("succeeded".to_string(), Some(sig.to_string())),
            Self::Excluded(err) => ("excluded".to_string(), Some(err.to_string())),
        }
    }

    fn try_from_raw(value: &str, inner_value: Option<String>) -> Result<Self> {
        let status = match (value, inner_value) {
            ("unprocessed", None) => Self::Unprocessed,
            ("disqualified", None) => Self::Disqualified,
            ("qualified", None) => Self::Qualified,
            ("unconfirmed", Some(sig)) => Self::Unconfirmed(Signature::from_str(&sig)?),
            ("failed", Some(err)) => Self::Failed(err),
            ("succeeded", Some(sig)) => Self::Succeeded(Signature::from_str(&sig)?),
            ("excluded", Some(err)) => Self::Excluded(err),
            (value, inner_value) => {
                panic!("Wrong arg was given to Status::try_from_raw: ({value}, {inner_value:?})")
            }
        };
        Ok(status)
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_record().0)
    }
}

#[derive(Debug, Default)]
pub struct WalletListEntry {
    pub wallet_pubkey: Pubkey,
    pub amount_to_airdrop: u64,
    pub ata: Option<Pubkey>,
    pub status: Status,
}

impl CsvEntrySer for WalletListEntry {
    fn to_record(&self) -> Vec<String> {
        let (status, status_inner) = self.status.to_record();
        vec![
            self.wallet_pubkey.to_string(),
            self.amount_to_airdrop.to_string(),
            self.ata.map(|pk| pk.to_string()).unwrap_or("".to_string()),
            status.to_string(),
            status_inner.unwrap_or("".to_string()),
        ]
    }
}

impl WalletListEntry {
    // Failed -> given status
    fn set_failed_to(&mut self, status: Status) {
        if let Status::Failed(_) = self.status {
            self.status = status;
        }
    }

    // Failed -> Excluded
    fn set_failed_to_excluded(&mut self) {
        if let Status::Failed(err) = &self.status {
            self.status = Status::Excluded(err.to_owned());
        }
    }

    fn set_unconfirmed_to_failed(&mut self) {
        if let Status::Unconfirmed(sig) = &self.status {
            self.status = Status::Failed(format!("{sig:?}: Could not confirm transaction"));
        }
    }

    fn set_unconfirmed_to_succeeded(&mut self) {
        if let Status::Unconfirmed(sig) = &self.status {
            self.status = Status::Succeeded(sig.to_owned());
        }
    }

    fn find_ata(&mut self, token_mint_pubkey: &Pubkey, token_program_id: &Pubkey) {
        if self.ata.is_none() {
            self.ata = Some(get_associated_token_address_with_program_id(
                &self.wallet_pubkey,
                token_mint_pubkey,
                token_program_id,
            ));
        }
    }

    pub fn to_transfer_ix(
        &self,
        token_mint_pubkey: &Pubkey,
        token_program_id: &Pubkey,
        token_decimals: u8,
        source_ata: &Pubkey,
        payer: &dyn Signer,
    ) -> Instruction {
        transfer_checked(
            token_program_id,
            source_ata,
            token_mint_pubkey,
            &self.ata.unwrap(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            self.amount_to_airdrop,
            token_decimals,
        )
        .unwrap_or_else(|_| {
            // NOTE:
            //  - normally, this should never happen since transfer_checked can only error out when incorrect program id was given.
            //  - if this errors out, then given data of wallet list has to contain a wrong line.
            panic!("This should not happen unless given wallet list data was wrong: {self:?}");
        })
    }
}

impl TryFrom<WalletListEntryRaw> for WalletListEntry {
    type Error = Error;

    fn try_from(
        WalletListEntryRaw {
            wallet_pubkey,
            amount_to_airdrop,
            ata,
            status,
            status_inner,
        }: WalletListEntryRaw,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        let wallet_pubkey = Pubkey::from_str(&wallet_pubkey)?;
        let ata = ata.and_then(|v| Pubkey::from_str(&v).ok()); // NOTE: if ata is somehow wrong, just set it to None and retry
        let status = Status::try_from_raw(
            &status.unwrap_or_else(|| Status::default().to_record().0),
            status_inner,
        )?;
        Ok(Self {
            wallet_pubkey,
            amount_to_airdrop,
            ata,
            status,
        })
    }
}

#[derive(Debug)]
pub struct WalletList(pub Vec<WalletListEntry>);

impl CsvListSerde for WalletList {
    fn parse_list_from_path(path: &PathBuf) -> Result<Self> {
        log::info!("Parsing wallet list from {path:?} ...");
        let data = std::fs::read_to_string(path)?;
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(false)
            .from_reader(data.as_bytes());
        let list = rdr
            .deserialize()
            .collect::<std::result::Result<Vec<WalletListEntryRaw>, _>>()?;
        let mut list = list
            .into_iter()
            .map(WalletListEntry::try_from)
            .collect::<std::result::Result<Vec<WalletListEntry>, _>>()?;
        list.sort_by(|a, b| a.wallet_pubkey.cmp(&b.wallet_pubkey));
        log::info!("Finished parsing wallet list");
        Ok(Self(list))
    }

    fn save_to_path(&mut self, path: &PathBuf) -> Result<()> {
        log::info!("Saving status data to {path:?} ...");
        log::info!("{:#?}", self.count_each_status());
        create_backup_if_file_exists(path)?;
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

impl WalletList {
    pub fn count_each_status(&self) -> HashMap<String, usize> {
        self.0.iter().fold(HashMap::new(), |mut map, entry| {
            map.entry(entry.status.to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);
            map
        })
    }

    pub fn count_qualified(&self) -> usize {
        self.0
            .iter()
            .filter(|entry| matches!(entry.status, Status::Qualified))
            .collect::<Vec<_>>()
            .len()
    }

    pub fn count_failed(&self) -> usize {
        self.0
            .iter()
            .filter(|entry| matches!(entry.status, Status::Failed(_)))
            .collect::<Vec<_>>()
            .len()
    }

    // DELETEME
    pub fn count_unconfirmed(&self) -> usize {
        self.0
            .iter()
            .filter(|entry| matches!(entry.status, Status::Unconfirmed(_)))
            .collect::<Vec<_>>()
            .len()
    }

    // Failed -> Unprocessed
    // used for retrying check_unprocessed procedure
    pub fn set_failed_to_unprocessed(&mut self) {
        for entry in self.0.iter_mut() {
            entry.set_failed_to(Status::Unprocessed);
        }
    }

    // Failed -> Qualified
    // used for retrying transfer_airdrop procedure
    pub fn set_failed_to_qualified(&mut self) {
        for entry in self.0.iter_mut() {
            entry.set_failed_to(Status::Qualified);
        }
    }

    // Failed -> Excluded
    // used for excluding failed entries
    pub fn set_failed_to_excluded(&mut self) {
        for entry in self.0.iter_mut() {
            entry.set_failed_to_excluded();
        }
    }

    // Unprocessed -> Qualified | Disqualified | Failed
    pub fn check_unprocessed(
        &mut self,
        rpc_client: &RpcClient,
        token_mint_pubkey: &Pubkey,
        token_program_id: &Pubkey,
        token_decimals: u8,
    ) {
        log::debug!("Checking qualification ...");
        for entries in self
            .0
            .iter_mut()
            .filter(|entry| matches!(entry.status, Status::Unprocessed))
            .collect::<Vec<_>>()
            .chunks_mut(ATA_GET_MULT_ACC_CHUNK_SIZE)
        {
            let atas: Vec<Pubkey> = entries
                .iter_mut()
                .map(|entry| {
                    entry.find_ata(token_mint_pubkey, token_program_id);
                    // UNWRAP-SAFTY: entry.find_ata is guaranteed to set entry.ata
                    entry.ata.unwrap()
                })
                .collect();
            let statuses = check_atas(rpc_client, &atas, token_decimals);
            for (entry, status) in entries.iter_mut().zip(statuses) {
                entry.status = status;
            }
        }
    }

    // Qualified -> Succeeded | Failed
    // NOTE: Failed status might contain false positive (rpc returned failure but token transfer happened)
    #[allow(clippy::too_many_arguments)]
    pub fn transfer_airdrop(
        &mut self,
        rpc_client: &RpcClient,
        token_mint_pubkey: &Pubkey,
        token_program_id: &Pubkey,
        token_decimals: u8,
        source_ata: &Pubkey,
        payer: &dyn Signer,
        compute_unit_limit: u32,
        compute_unit_price: u64,
        dry_run: bool,
        should_confirm: bool,
    ) {
        let transfer_ixs_with_idx: Vec<(usize, Instruction)> = self
            .0
            .iter()
            .filter(|entry| entry.wallet_pubkey != payer.pubkey())
            .enumerate()
            .filter_map(|(idx, entry)| match entry.status {
                Status::Qualified => {
                    let ix = entry.to_transfer_ix(
                        token_mint_pubkey,
                        token_program_id,
                        token_decimals,
                        source_ata,
                        payer,
                    );
                    Some((idx, ix))
                }
                _ => None,
            })
            .collect();

        log::info!(
            "Sending {} txs ...",
            transfer_ixs_with_idx
                .len()
                .div_ceil(TRANSFER_IXS_CHUNK_SIZE)
        );
        let compute_budget_ixs = get_compute_budget_ixs(compute_unit_limit, compute_unit_price);
        for ixs_with_idx in transfer_ixs_with_idx.chunks(TRANSFER_IXS_CHUNK_SIZE) {
            let (idxs, transfer_ixs): (Vec<_>, Vec<_>) = ixs_with_idx.iter().cloned().unzip();

            let ixs: Vec<Instruction> = compute_budget_ixs
                .iter()
                .cloned()
                .chain(transfer_ixs)
                .collect();

            // TODO: error handling and retry
            let tx = prep_tx(rpc_client, payer, &ixs).unwrap();

            if dry_run {
                log::info!("{:#?}", rpc_client.simulate_transaction(&tx).unwrap());
            } else {
                let status = if should_confirm {
                    let _tx_res = rpc_client
                        .send_and_confirm_transaction_with_spinner_and_commitment(
                            &tx,
                            rpc_client.commitment(),
                        );
                    // NOTE: just set it to unconfirmed to be safe (always manually run the confirm stage to resolve)
                    Status::Unconfirmed(tx.get_signature().to_owned())
                } else {
                    let tx_res = rpc_client.send_transaction(&tx);
                    match tx_res {
                        Ok(sig) => Status::Unconfirmed(sig),
                        Err(err) => Status::Failed(err.to_string()),
                    }
                };
                for idx in idxs {
                    self.0.get_mut(idx).unwrap().status = status.clone();
                }
            }
        }
    }

    pub fn get_unconfirmed_sigs(&self) -> HashSet<Signature> {
        self.0
            .iter()
            .filter(|entry| matches!(entry.status, Status::Unconfirmed(_)))
            .map(|entry| match entry.status {
                Status::Unconfirmed(sig) => sig,
                _ => unreachable!(),
            })
            .collect()
    }

    // Unconfirmed -> Succeeded | Unconfirmed
    // returns number of unconfirmed sigs
    pub fn confirm(&mut self, rpc_client: &RpcClient) -> usize {
        let unconfirmed_signatures = self.get_unconfirmed_sigs();

        let unconfirmed_count = unconfirmed_signatures.len();
        log::debug!("Confirming {} txs ...", unconfirmed_count);
        let mut confirmed_count: usize = 0;
        for sig in unconfirmed_signatures {
            let res = rpc_client.confirm_transaction_with_commitment(&sig, rpc_client.commitment());
            if let Ok(response) = res {
                if response.value {
                    log::debug!("Confirmed: {sig:?}");
                    self.0
                        .iter_mut()
                        .filter(|entry| match entry.status {
                            Status::Unconfirmed(signature) => signature == sig,
                            _ => false,
                        })
                        .for_each(|entry| entry.set_unconfirmed_to_succeeded());
                    confirmed_count += 1;
                } else {
                    log::debug!("Unconfirmed: {sig:?}");
                }
            } else {
                log::debug!("Failed to get tx: {sig:?}");
                // TODO: should this set the status to failed?
            }
        }
        let unconfirmed_count = unconfirmed_count - confirmed_count;
        log::debug!(
            "Confirmed: {}; Unconfirmed: {}",
            confirmed_count,
            unconfirmed_count
        );
        unconfirmed_count
    }

    // Unconfirmed -> Failed
    pub fn set_unconfirmed_to_failed(&mut self) {
        for entry in self.0.iter_mut() {
            entry.set_unconfirmed_to_failed();
        }
    }
}

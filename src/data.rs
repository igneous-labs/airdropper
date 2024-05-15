use std::{path::Path, str::FromStr};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Signature, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::instruction::transfer_checked;

use crate::{
    consts::{ATA_GET_MULT_ACC_CHUNK_SIZE, TRANSFER_IXS_CHUNK_SIZE},
    errors::{Error, Result},
    utils::{check_atas, get_compute_budget_ixs, prep_tx},
};

// TODO: use serde with
#[derive(Debug, serde::Deserialize, Clone)]
struct WalletListEntryRaw {
    pub wallet_pubkey: String,
    pub amount_to_airdrop: u64,
    pub ata: Option<String>,
    pub status: Option<u8>,
    pub status_inner: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub enum Status {
    #[default]
    Unprocessed,
    Disqualified,
    Qualified,
    Failed(String), // marked to be reset to Unprocessed for retrial
    Succeeded(Signature),
    Excluded(String), // set aside due to too many failed attempts
}

impl Status {
    fn to_record(&self) -> (u8, Option<String>) {
        match self {
            Self::Unprocessed => (0, None),
            Self::Disqualified => (1, None),
            Self::Qualified => (2, None),
            Self::Failed(err) => (3, Some(err.to_string())),
            Self::Succeeded(sig) => (4, Some(sig.to_string())),
            Self::Excluded(err) => (5, Some(err.to_string())),
        }
    }

    fn try_from_raw(value: u8, inner_value: Option<String>) -> Result<Self> {
        let status = match (value, inner_value) {
            (0, None) => Self::Unprocessed,
            (1, None) => Self::Disqualified,
            (2, None) => Self::Qualified,
            (3, Some(err)) => Self::Failed(err),
            (4, Some(sig)) => Self::Succeeded(Signature::from_str(&sig)?),
            (5, Some(err)) => Self::Excluded(err),
            (value, inner_value) => {
                panic!("Wrong arg was given to Status::try_from_raw: ({value}, {inner_value:?})")
            }
        };
        Ok(status)
    }
}

#[derive(Debug, Default)]
pub struct WalletListEntry {
    pub wallet_pubkey: Pubkey,
    pub amount_to_airdrop: u64,
    pub ata: Option<Pubkey>,
    pub status: Status,
}

impl WalletListEntry {
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

    // Failed -> given status
    fn set_failed(&mut self, status: Status) {
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

    fn find_ata(&mut self, token_mint_pubkey: &Pubkey, token_program_id: &Pubkey) {
        if self.ata.is_none() {
            self.ata = Some(get_associated_token_address_with_program_id(
                &self.wallet_pubkey,
                &token_mint_pubkey,
                &token_program_id,
            ));
        }
    }

    // if status = Qualified, then prep transfer ix
    pub fn to_transfer_ix(
        &self,
        token_mint_pubkey: &Pubkey,
        token_program_id: &Pubkey,
        token_decimals: u8,
        source_ata: &Pubkey,
        payer: &dyn Signer,
    ) -> Option<Instruction> {
        if let Status::Qualified = self.status {
            let ix = transfer_checked(
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
            });

            Some(ix)
        } else {
            None
        }
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
            status.unwrap_or_else(|| Status::default().to_record().0),
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

impl WalletList {
    pub fn parse_list_from_path<T: AsRef<Path>>(path: T) -> Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(false)
            .from_reader(data.as_bytes());
        let list = rdr
            .deserialize()
            .collect::<std::result::Result<Vec<WalletListEntryRaw>, _>>()?;
        let list = list
            .into_iter()
            .map(WalletListEntry::try_from)
            .collect::<std::result::Result<Vec<WalletListEntry>, _>>()?;
        Ok(Self(list))
    }

    pub fn save_to_path<T: AsRef<Path>>(&self, path: T) -> Result<()> {
        let mut wtr = csv::Writer::from_path(path)?;
        for entry in self.0.iter() {
            wtr.write_record(entry.to_record())?;
        }
        wtr.flush()?;
        Ok(())
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

    // Failed -> Unprocessed
    // used for retrying check_unprocessed procedure
    pub fn set_failed_to_unprocessed(&mut self) {
        for entry in self.0.iter_mut() {
            entry.set_failed(Status::Unprocessed);
        }
    }

    // Failed -> Qualified
    // used for retrying transfer_airdrop procedure
    pub fn set_failed_to_qualified(&mut self) {
        for entry in self.0.iter_mut() {
            entry.set_failed(Status::Qualified);
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
        log::debug!("Finding atas ...");
        for entry in self.0.iter_mut() {
            entry.find_ata(token_mint_pubkey, token_program_id);
        }
        log::debug!("Checking qualification ...");
        for entries in self.0.chunks_mut(ATA_GET_MULT_ACC_CHUNK_SIZE) {
            let atas: Vec<Pubkey> = entries.iter().map(|entry| entry.ata.unwrap()).collect();
            let statuses = check_atas(rpc_client, &atas, token_decimals);
            for (entry, status) in entries.iter_mut().zip(statuses) {
                entry.status = status;
            }
        }
    }

    // Qualified -> Succeeded | Failed
    // NOTE: Failed status might contain false positive (rpc returned failure but token transfer happened)
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
    ) {
        let transfer_ixs_with_idx: Vec<(usize, Instruction)> = self
            .0
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                entry
                    .to_transfer_ix(
                        token_mint_pubkey,
                        token_program_id,
                        token_decimals,
                        source_ata,
                        payer,
                    )
                    .map(|ix| (idx, ix))
            })
            .collect();

        // TODO: proper compute budget args
        let compute_budget_ixs = get_compute_budget_ixs(compute_unit_limit, compute_unit_price);
        for ixs_with_idx in transfer_ixs_with_idx.chunks(TRANSFER_IXS_CHUNK_SIZE) {
            let (idxs, transfer_ixs): (Vec<_>, Vec<_>) = ixs_with_idx.iter().cloned().unzip();

            let ixs: Vec<Instruction> = compute_budget_ixs
                .iter()
                .cloned()
                .chain(transfer_ixs)
                .collect();
            let tx = prep_tx(rpc_client, payer, &ixs);

            if dry_run {
                log::info!("{:#?}", rpc_client.simulate_transaction(&tx).unwrap());
            } else {
                let tx_res = rpc_client.send_and_confirm_transaction_with_spinner_and_commitment(
                    &tx,
                    rpc_client.commitment(),
                );

                let status = match tx_res {
                    Ok(sig) => Status::Succeeded(sig),
                    Err(err) => Status::Failed(err.to_string()),
                };
                for idx in idxs {
                    self.0.get_mut(idx).unwrap().status = status.clone();
                }
            }
        }
    }
}

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use bytemuck::try_from_bytes;
use clap::{
    builder::{StringValueParser, TypedValueParser},
    Args,
};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair_file,
    signer::Signer,
};

use crate::{
    consts::DEFAULT_SNAPSHOT_MINIMUM_BALANCE_ATOMIC,
    data::{CsvListSerde, Snapshot, SnapshotEntry},
    errors::{Error, Result},
    subcmd::Subcmd,
    utils::get_token_mint_info,
};

const MINT_OFFSET: usize = 0;
const OWNER_OFFSET: usize = 32;
const OWNER_LENGTH: usize = 32;
const AMOUNT_LENGTH: usize = 8;

const TOKEN_ACCOUNT_SIZE: u64 = 165;

#[derive(Args, Debug)]
#[command(long_about = "Take a token snapshot of given mint")]
pub struct SnapshotArgs {
    #[arg(
        long,
        short,
        help = "Mint pubkey of the token to be snapshotted",
        value_parser = StringValueParser::new().try_map(|s| Pubkey::from_str(&s)),
    )]
    snapshot_token_mint_pubkey: Pubkey,

    #[arg(
        long,
        short,
        help = "The required minimum balance (in token atomic) for snapshot",
        default_value_t = DEFAULT_SNAPSHOT_MINIMUM_BALANCE_ATOMIC,
    )]
    minimum_balance: u64,

    #[arg(
        long,
        short,
        help = "Path to payer keypair who holds the token to be airdropped (to be excluded from snapshot)"
    )]
    payer_path: Option<PathBuf>,

    #[arg(long, short, help = "Pubkeys to exclude from snapshot")]
    black_list: Vec<String>,

    #[arg(long, short, help = "Path to token snapshot csv file")]
    snapshot_path: PathBuf,
}

impl SnapshotArgs {
    pub fn run(args: crate::Args) -> Result<()> {
        let Self {
            snapshot_token_mint_pubkey,
            minimum_balance,
            payer_path,
            black_list,
            snapshot_path,
        } = match args.subcmd {
            Subcmd::Snapshot(a) => a,
            _ => unreachable!(),
        };
        let mut black_list = black_list
            .into_iter()
            .map(|pk_str| Pubkey::from_str(&pk_str).map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;

        if let Some(payer_path) = payer_path {
            let payer_pubkey = read_keypair_file(
                payer_path
                    .to_str()
                    .expect("Could not convert payer_path to str"),
            )
            .map_err(|_e| Error::KeyPairError)?
            .pubkey();
            black_list.push(payer_pubkey);
        }
        log::info!("Required minimum balance: {}", minimum_balance);

        log::info!(
            "Taking token snapshot for {:?}...",
            snapshot_token_mint_pubkey
        );
        let rpc_client = args.config.rpc_client();

        let mut snapshot = take_snapshot(
            &rpc_client,
            &snapshot_token_mint_pubkey,
            minimum_balance,
            &black_list,
        )?;
        log::info!("Total fetched wallet count: {}", snapshot.0.len());

        if !args.dry_run {
            snapshot
                .save_to_path(&snapshot_path)
                .unwrap_or_else(|err| log::error!("Failed to save snapshot: {err:?}"));
        }

        Ok(())
    }
}

pub fn take_snapshot(
    rpc_client: &RpcClient,
    token_mint_pubkey: &Pubkey,
    minimum_balance_atomic: u64,
    blacklist: &[Pubkey],
) -> Result<Snapshot> {
    let (token_program_id, _token_decimals) = get_token_mint_info(rpc_client, token_mint_pubkey)?;

    let filters = {
        let by_mint = RpcFilterType::Memcmp(Memcmp::new(
            MINT_OFFSET, // mint
            MemcmpEncodedBytes::Base58(token_mint_pubkey.to_string()),
        ));
        let by_datasize = RpcFilterType::DataSize(TOKEN_ACCOUNT_SIZE);
        vec![by_datasize, by_mint]
    };

    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            // Fetch owner pubkey (32 +32), and amount (64 +8)
            data_slice: Some(UiDataSliceConfig {
                offset: OWNER_OFFSET,
                length: OWNER_LENGTH + AMOUNT_LENGTH,
            }),
            commitment: Some(CommitmentConfig::processed()),
            min_context_slot: None,
        },
        with_context: None,
    };

    let mut entries = HashMap::new();
    rpc_client
        .get_program_accounts_with_config(&token_program_id, config)?
        .into_iter()
        .for_each(|(_token_account_pubkey, account)| {
            let wallet_pubkey: Pubkey = *try_from_bytes(&account.data[..OWNER_LENGTH]).unwrap();
            if blacklist.contains(&wallet_pubkey) {
                return;
            }
            let token_balance_atomic: u64 =
                *try_from_bytes(&account.data[OWNER_LENGTH..OWNER_LENGTH + AMOUNT_LENGTH]).unwrap();
            if token_balance_atomic >= minimum_balance_atomic {
                entries
                    .entry(wallet_pubkey)
                    .and_modify(|e| *e += token_balance_atomic)
                    .or_insert(token_balance_atomic);
            }
        });
    Ok(Snapshot(
        entries
            .into_iter()
            .map(|(wallet_pubkey, token_balance_atomic)| SnapshotEntry {
                wallet_pubkey,
                token_balance_atomic,
            })
            .collect(),
    ))
}

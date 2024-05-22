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
    data::{WalletList, WalletListEntry},
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
#[command(
    long_about = "Take a token snapshot of given mint and generate wallet list for the airdrop"
)]
pub struct SnapshotArgs {
    #[arg(
        long,
        short,
        help = "Mint pubkey of the token to be snapshotted",
        value_parser = StringValueParser::new().try_map(|s| Pubkey::from_str(&s)),
    )]
    snapshot_token_mint_pubkey: Pubkey,

    #[arg(long, short, help = "The total amount (in token atomic) to air drop")]
    amount_to_airdrop: u64,

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
    payer_path: PathBuf,
}

impl SnapshotArgs {
    pub fn run(args: crate::Args) -> Result<()> {
        let Self {
            snapshot_token_mint_pubkey,
            amount_to_airdrop,
            minimum_balance,
            payer_path,
        } = match args.subcmd {
            Subcmd::Snapshot(a) => a,
            _ => unreachable!(),
        };
        let payer_pubkey = read_keypair_file(
            payer_path
                .to_str()
                .expect("Could not convert payer_path to str"),
        )
        .map_err(|_e| Error::KeyPairError)?
        .pubkey();

        log::info!("Required minimum balance: {}", minimum_balance);
        log::info!("Total amount to airdrop: {}", amount_to_airdrop);

        log::info!(
            "Taking token snapshot for {:?}...",
            snapshot_token_mint_pubkey
        );
        let rpc_client = args.config.rpc_client();

        let snapshot = take_snapshot(
            &rpc_client,
            &snapshot_token_mint_pubkey,
            minimum_balance,
            &[payer_pubkey],
        )?;
        log::info!("Total fetched wallet count: {}", snapshot.len());

        let total_amount: u64 = snapshot.values().sum();
        let mut wallet_list = WalletList(
            snapshot
                .into_iter()
                .filter_map(|(wallet_pubkey, balance)| {
                    let amount_to_airdrop =
                        (balance as u128 * amount_to_airdrop as u128 / total_amount as u128) as u64;
                    if amount_to_airdrop != 0 {
                        Some(WalletListEntry {
                            wallet_pubkey,
                            amount_to_airdrop,
                            ..Default::default()
                        })
                    } else {
                        None
                    }
                })
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
                .save_to_path(&args.wallet_list_path)
                .unwrap_or_else(|err| log::error!("Failed to save status list: {err:?}"));
        }

        Ok(())
    }
}

pub fn take_snapshot(
    rpc_client: &RpcClient,
    token_mint_pubkey: &Pubkey,
    minimum_balance_atomic: u64,
    blacklist: &[Pubkey],
) -> Result<HashMap<Pubkey, u64>> {
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

    let mut res = HashMap::new();
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
                res.entry(wallet_pubkey)
                    .and_modify(|e| *e += token_balance_atomic)
                    .or_insert(token_balance_atomic);
            }
        });
    Ok(res)
}

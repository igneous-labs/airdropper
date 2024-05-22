use std::{io::Write, path::PathBuf};

use solana_account_decoder::parse_token::{parse_token, TokenAccountType};
use solana_client::rpc_client::RpcClient;
use solana_program::instruction::Instruction;
use solana_sdk::{
    account::Account,
    compute_budget::ComputeBudgetInstruction,
    message::{v0::Message, VersionedMessage},
    pubkey::Pubkey,
    signer::Signer,
    transaction::VersionedTransaction,
};
use spl_token_2022::{extension::StateWithExtensionsOwned, state::Mint};

use crate::{data::Status, errors::Result};

// check if given token_account is qualified for airdrop
// returns Qualified | Disqualified
fn qualification_predicate(token_account: &Option<Account>, token_decimals: u8) -> Status {
    match token_account {
        Some(account) => {
            let token_account = parse_token(&account.data, Some(token_decimals));
            match token_account {
                Ok(TokenAccountType::Account(_ui_token_account)) => {
                    // NB: check additional qualifications
                    // NOTE: for now just having an account qualifies for airdrop
                    Status::Qualified
                }
                _ => Status::Disqualified,
            }
        }
        None => Status::Disqualified,
    }
}

// Failed | Qualified | Disqualified
pub fn check_atas(rpc_client: &RpcClient, atas: &[Pubkey], token_decimals: u8) -> Vec<Status> {
    let res = rpc_client.get_multiple_accounts_with_commitment(atas, rpc_client.commitment());
    match res {
        Ok(response) => response
            .value
            .iter()
            .map(|token_account| qualification_predicate(token_account, token_decimals))
            .collect(),
        Err(err) => vec![Status::Failed(err.to_string()); atas.len()],
    }
}

/// Returns (token_program_id: Pubkey, decimals: u8)
pub fn get_token_mint_info(
    rpc_client: &RpcClient,
    token_mint_pubkey: &Pubkey,
) -> Result<(Pubkey, u8)> {
    let Account { owner, data, .. } = rpc_client.get_account(token_mint_pubkey)?;
    let token_program_id = owner;
    let token_decimals = StateWithExtensionsOwned::<Mint>::unpack(data)?
        .base
        .decimals;

    Ok((token_program_id, token_decimals))
}

/// prepare transaction with given ixs
pub fn prep_tx(
    rpc_client: &RpcClient,
    payer: &dyn Signer,
    ixs: &[Instruction],
) -> VersionedTransaction {
    let rbh = rpc_client.get_latest_blockhash().unwrap();
    VersionedTransaction::try_new(
        VersionedMessage::V0(Message::try_compile(&payer.pubkey(), ixs, &[], rbh).unwrap()),
        &[payer],
    )
    .unwrap()
}

pub fn get_compute_budget_ixs(
    compute_unit_limit: u32,
    compute_unit_price: u64,
) -> [Instruction; 2] {
    [
        ComputeBudgetInstruction::set_compute_unit_limit(compute_unit_limit),
        ComputeBudgetInstruction::set_compute_unit_price(compute_unit_price),
    ]
}

pub fn add_to_filename(path: &PathBuf, name: &str) -> PathBuf {
    let mut res = path.clone();
    let stem = res.file_stem().unwrap().to_str().unwrap();
    let ext = res.extension().unwrap().to_str().unwrap();
    res.set_file_name(format!("{stem}.{name}.{ext}"));
    res
}

// intended to be used to save stage results without overwriting.
// i.e. given "wallet-list.checked.csv" and "wallet-list.checked.0.csv" exists,
// the function moves "wallet-list.checked.csv" to "wallet-list.checked.1.csv"
pub fn create_backup_if_file_exists(path: &PathBuf) -> Result<()> {
    if !path.try_exists()? {
        return Ok(());
    }

    let mut n = 0;
    let backup_path = loop {
        let target = add_to_filename(path, &n.to_string());
        if !target.try_exists()? {
            break target;
        }
        n += 1;
    };

    log::info!("Saving backup for {path:?} to {backup_path:?}");
    std::fs::rename(path, backup_path)?;

    Ok(())
}

// prompt for confirmation for a potentially mistakable action
pub fn prompt_confirmation(msg: &str) -> bool {
    let mut buffer = String::new();
    loop {
        print!("{msg} (Y/N): ");
        let _ = std::io::stdout().flush();
        let _ = std::io::stdin().read_line(&mut buffer);
        break buffer.trim().to_uppercase().as_str() == "Y";
    }
}

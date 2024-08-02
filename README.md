# Airdropper

An airdrop cli helper tool.

```
Usage: airdropper [OPTIONS] --wallet-list-path <WALLET_LIST_PATH> <COMMAND>

Commands:
  snapshot  Take a token snapshot of given mint and generate wallet list for the airdrop
  check     Given wallet list, check associated token accounts
  send      Send airdrop transactions
  confirm   Confirm unconfirmed entries
  display   Display wallet list content
  help      Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>
          Path to solana CLI config. Defaults to solana cli default if not provided [default: ]
  -w, --wallet-list-path <WALLET_LIST_PATH>
          Path to wallet_list csv file in the format of "wallet_pubkey,amount_to_airdrop"
  -d, --dry-run

  -h, --help
          Print help
  -V, --version
          Print version
```


## Airdrop Procedure

1. [snapshot](/#1.-Snapshot)
1. [check](/#2.-Check)
1. [send](/#3.-Send)
1. [confirm](/#4.-Confirm)

### 1. Snapshot

```
Take a token snapshot of given mint and generate wallet list for the airdrop

Usage: airdropper --wallet-list-path <WALLET_LIST_PATH> snapshot [OPTIONS] --snapshot-token-mint-pubkey <SNAPSHOT_TOKEN_MINT_PUBKEY> --amount-to-airdrop <AMOUNT_TO_AIRDROP> --payer-path <PAYER_PATH>

Options:
  -s, --snapshot-token-mint-pubkey <SNAPSHOT_TOKEN_MINT_PUBKEY>
          Mint pubkey of the token to be snapshotted

  -a, --amount-to-airdrop <AMOUNT_TO_AIRDROP>
          The total amount (in token atomic) to air drop

  -m, --minimum-balance <MINIMUM_BALANCE>
          The required minimum balance (in token atomic) for snapshot

          [default: 1]

  -p, --payer-path <PAYER_PATH>
          Path to payer keypair who holds the token to be airdropped (to be excluded from snapshot)

  -h, --help
          Print help (see a summary with '-h')
```


### 2. Check

```
Given wallet list, check associated token accounts

Usage: airdropper --wallet-list-path <WALLET_LIST_PATH> check --airdrop-token-mint-pubkey <AIRDROP_TOKEN_MINT_PUBKEY>

Options:
  -a, --airdrop-token-mint-pubkey <AIRDROP_TOKEN_MINT_PUBKEY>
          Mint pubkey of the token to be airdropped

  -h, --help
          Print help (see a summary with '-h')
```

### 3. Send

```
Send airdrop transactions

Usage: airdropper --wallet-list-path <WALLET_LIST_PATH> send [OPTIONS] --airdrop-token-mint-pubkey <AIRDROP_TOKEN_MINT_PUBKEY> --payer-path <PAYER_PATH>

Options:
  -a, --airdrop-token-mint-pubkey <AIRDROP_TOKEN_MINT_PUBKEY>
          Mint pubkey of the token to be airdropped

  -p, --payer-path <PAYER_PATH>
          Path to payer keypair who holds the token to be airdropped

  -l, --compute-unit-limit <COMPUTE_UNIT_LIMIT>
          Compute unit limit

          [default: 1000000]

  -p, --compute-unit-price <COMPUTE_UNIT_PRICE>
          Compute unit price in micro lamports

          [default: 1]

  -s, --should-confirm
          After sending transaction, wait for confirmation before proceeding

  -h, --help
          Print help (see a summary with '-h')
```

### 4. Confirm

```
Confirm unconfirmed entries

Usage: airdropper --wallet-list-path <WALLET_LIST_PATH> confirm

Options:
  -h, --help
          Print help (see a summary with '-h')
```

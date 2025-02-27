# Airdropper

An airdrop cli helper tool.

```
Usage: airdropper [OPTIONS] <COMMAND>

Commands:
  snapshot     Take a token snapshot of given mint
  wallet-list  Given a token snapshot and a airdrop amount, generate a wallet list
  check        Given a wallet list, check qualification of each entry
  send         Given a checked wallet list, send airdrop transactions
  confirm      Given a sent wallet list, confirm unconfirmed transactions
  display      Display wallet list content
  help         Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  Path to solana CLI config. Defaults to solana cli default if not provided [default: ]
  -d, --dry-run          dry run (note: if set, does not save any files nor send any transactions)
  -h, --help             Print help
  -V, --version          Print version
```


## Airdrop Procedure

1. [snapshot](/#1.-Snapshot)
1. [wallet-list](/#2.-Wallet-List)
1. [check](/#3.-Check)
1. [send](/#4.-Send)
1. [confirm](/#5.-Confirm)

### 1. Snapshot

```
Take a token snapshot of given mint

Usage: airdropper snapshot [OPTIONS] --snapshot-token-mint-pubkey <SNAPSHOT_TOKEN_MINT_PUBKEY> --snapshot-path <SNAPSHOT_PATH>

Options:
  -s, --snapshot-token-mint-pubkey <SNAPSHOT_TOKEN_MINT_PUBKEY>
          Mint pubkey of the token to be snapshotted

  -m, --minimum-balance <MINIMUM_BALANCE>
          The required minimum balance (in token atomic) for snapshot

          [default: 1]

  -p, --payer-path <PAYER_PATH>
          Path to payer keypair who holds the token to be airdropped (to be excluded from snapshot)

  -b, --black-list <BLACK_LIST>
          Pubkeys to exclude from snapshot

  -s, --snapshot-path <SNAPSHOT_PATH>
          Path to token snapshot csv file

  -h, --help
          Print help (see a summary with '-h')
```


### 2. Wallet List
```
Given a token snapshot and a airdrop amount, generate a wallet list

Usage: airdropper wallet-list --wallet-list-path <WALLET_LIST_PATH> --amount-to-airdrop <AMOUNT_TO_AIRDROP> --snapshot-path <SNAPSHOT_PATH>

Options:
  -w, --wallet-list-path <WALLET_LIST_PATH>
          Path to wallet list csv file

  -a, --amount-to-airdrop <AMOUNT_TO_AIRDROP>
          The total amount (in token atomic) to airdrop

  -s, --snapshot-path <SNAPSHOT_PATH>
          Path to token snapshot csv file

  -h, --help
          Print help (see a summary with '-h')
```


### 3. Check

```
Given a wallet list, check qualification of each entry

Usage: airdropper check --wallet-list-path <WALLET_LIST_PATH> --airdrop-token-mint-pubkey <AIRDROP_TOKEN_MINT_PUBKEY>

Options:
  -w, --wallet-list-path <WALLET_LIST_PATH>
          Path to wallet list csv file

  -a, --airdrop-token-mint-pubkey <AIRDROP_TOKEN_MINT_PUBKEY>
          Mint pubkey of the token to be airdropped

  -h, --help
          Print help (see a summary with '-h')
```

### 4. Send

```
Given a checked wallet list, send airdrop transactions

Usage: airdropper send [OPTIONS] --wallet-list-path <WALLET_LIST_PATH> --airdrop-token-mint-pubkey <AIRDROP_TOKEN_MINT_PUBKEY> --payer-path <PAYER_PATH>

Options:
  -w, --wallet-list-path <WALLET_LIST_PATH>
          Path to wallet list csv file

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

### 5. Confirm

```
Given a sent wallet list, confirm unconfirmed transactions

Usage: airdropper confirm --wallet-list-path <WALLET_LIST_PATH>

Options:
  -w, --wallet-list-path <WALLET_LIST_PATH>
          Path to wallet list csv file

  -h, --help
          Print help (see a summary with '-h')
```

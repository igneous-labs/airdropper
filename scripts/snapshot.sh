#!/usr/bin/env bash

readonly BIN="airdropper"
readonly BASE_DIR=$(dirname $(readlink -f "$0"))

# NB: token for snapshot
readonly SYMBOL="SYMBOL"
readonly MINT_PK="MINT_PUBKEY"

readonly PAYER_PATH="<PATH_TO_PAYER_KEYPAIR>"

readonly EPOCH=$(solana epoch)
readonly TS=$(date "+%s")

mkdir -p ${BASE_DIR}/${EPOCH}
${BIN} snapshot \
  --payer-path ${PAYER_PATH} \
  --snapshot-token-mint-pubkey ${MINT_PK} \
  --snapshot-path ${BASE_DIR}/${EPOCH}/${TS}-${SYMBOL}-snapshot.csv

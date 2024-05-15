use derive_more::Display;
use solana_program::program_error::ProgramError;
use solana_rpc_client_api::client_error::Error as RpcError;
use solana_sdk::{pubkey::ParsePubkeyError, signature::ParseSignatureError};
use tokio::task::JoinError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Display)]
pub enum Error {
    IoError(std::io::Error),
    CsvError(csv::Error),
    PubkeyError(ParsePubkeyError),
    SignatureError(ParseSignatureError),
    RpcError(RpcError),
    ProgramError(ProgramError),
    JoinError(JoinError),
    KeyPairError,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<csv::Error> for Error {
    fn from(value: csv::Error) -> Self {
        Self::CsvError(value)
    }
}

impl From<ParsePubkeyError> for Error {
    fn from(value: ParsePubkeyError) -> Self {
        Self::PubkeyError(value)
    }
}

impl From<ParseSignatureError> for Error {
    fn from(value: ParseSignatureError) -> Self {
        Self::SignatureError(value)
    }
}

impl From<RpcError> for Error {
    fn from(value: RpcError) -> Self {
        Self::RpcError(value)
    }
}

impl From<ProgramError> for Error {
    fn from(value: ProgramError) -> Self {
        Self::ProgramError(value)
    }
}

impl From<JoinError> for Error {
    fn from(value: JoinError) -> Self {
        Self::JoinError(value)
    }
}

impl std::error::Error for Error {}

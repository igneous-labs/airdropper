use std::path::PathBuf;

use crate::errors::Result;

pub use snapshot::*;
pub use wallet_list::*;

mod snapshot;
mod wallet_list;

pub trait CsvListSerde: Sized {
    fn parse_list_from_path(path: &PathBuf) -> Result<Self>;
    fn save_to_path(&mut self, path: &PathBuf) -> Result<()>;
}

pub trait CsvEntrySer {
    fn to_record(&self) -> Vec<String>;
}

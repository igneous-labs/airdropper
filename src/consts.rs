pub const TRANSFER_IXS_CHUNK_SIZE: usize = 18;
pub const ATA_GET_MULT_ACC_CHUNK_SIZE: usize = 100;

pub const CHECK_MAX_RETRY: usize = 4;
pub const TRANSFER_MAX_RETRY: usize = 1; // For now, manually retry
pub const CONFIRM_TX_MAX_RETRY: usize = 3;

pub const CONFIRM_TX_SLEEP_SEC: u64 = 90;

pub const DEFAULT_COMPUTE_UNIT_LIMIT: u32 = 1_000_000;
pub const DEFAULT_COMPUTE_UNIT_PRICE: u64 = 1;

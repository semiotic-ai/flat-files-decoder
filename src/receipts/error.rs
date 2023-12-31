use crate::transactions::tx_type::TransactionTypeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReceiptError {
    #[error("Invalid status")]
    InvalidStatus,
    #[error("Invalid tx type")]
    InvalidTxType(#[from] TransactionTypeError),
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    #[error("Invalid topic: {0}")]
    InvalidTopic(String),
    #[error("Invalid data: {0}")]
    InvalidBloom(String),
    #[error("Receipt root mismatch: {0} != {1}")]
    MismatchedRoot(String, String),
    #[error("Missing receipt root")]
    MissingRoot,
    #[error("Missing receipt")]
    MissingReceipt,
}

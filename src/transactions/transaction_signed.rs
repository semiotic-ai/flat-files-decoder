use std::str::FromStr;

use crate::protos::block::{BigInt, TransactionTrace};
use crate::transactions::error::TransactionError;
use protobuf::MessageField;
use reth_primitives::{Signature, Transaction, TransactionSigned};
use revm_primitives::{B256, hex};

impl TryFrom<&TransactionTrace> for TransactionSigned {
    type Error = TransactionError;

    fn try_from(trace: &TransactionTrace) -> Result<Self, Self::Error> {
        let transaction = Transaction::try_from(trace)?;
        let signature = Signature::try_from(trace)?;
        let hash = B256::from_str(&hex::encode(trace.hash.as_slice())).map_err(|_| TransactionError::MissingCall)?;
        let tx_signed = TransactionSigned{
            transaction: transaction.clone(),
            signature: signature.clone(),
            hash,
        };
        Ok(tx_signed)
    }
}

pub fn u128_from_field(field: &MessageField<BigInt>) -> Result<u128, TransactionError> {
    field.get_or_default().clone().try_into()
}

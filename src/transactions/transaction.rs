use crate::transactions::access_list::compute_access_list;
use crate::transactions::error::TransactionError;
use crate::transactions::tx_type::map_tx_type;
use alloy_primitives::{FixedBytes, TxKind, Uint};
use reth_primitives::{
    Address, Bytes, ChainId, Transaction, TxEip1559, TxEip2930, TxEip4844, TxLegacy, TxType,
};
use sf_protos::ethereum::r#type::v2::{CallType, TransactionTrace};

use super::bigint_to_u128;

pub const CHAIN_ID: ChainId = 1;

pub fn trace_to_transaction(trace: &TransactionTrace) -> Result<Transaction, TransactionError> {
    let tx_type = map_tx_type(&trace.r#type)?;

    let nonce = trace.nonce;
    let gas_price = bigint_to_u128(trace.gas_price.clone().unwrap_or_default())?;
    let gas_limit = trace.gas_limit;

    let to = get_tx_kind(trace)?;

    let chain_id = CHAIN_ID;

    let value = Uint::from(bigint_to_u128(trace.value.clone().unwrap_or_default())?);
    let input = Bytes::copy_from_slice(trace.input.as_slice());

    let transaction: Transaction = match tx_type {
        TxType::Legacy => {
            let v: u8 = if trace.v.is_empty() { 0 } else { trace.v[0] };

            let chain_id: Option<ChainId> = if v == 27 || v == 28 {
                None
            } else {
                Some(CHAIN_ID)
            };

            Transaction::Legacy(TxLegacy {
                chain_id,
                nonce,
                gas_price,
                gas_limit,
                to,
                value,
                input,
            })
        }
        TxType::Eip2930 => {
            let access_list = compute_access_list(&trace.access_list)?;

            Transaction::Eip2930(TxEip2930 {
                chain_id,
                nonce,
                gas_price,
                gas_limit,
                to,
                value,
                access_list,
                input,
            })
        }
        TxType::Eip1559 => {
            let access_list = compute_access_list(&trace.access_list)?;

            let trace_max_fee_per_gas = trace.max_fee_per_gas.clone().unwrap_or_default();
            let max_fee_per_gas = bigint_to_u128(trace_max_fee_per_gas)?;

            let trace_max_priority_fee_per_gas =
                trace.max_priority_fee_per_gas.clone().unwrap_or_default();
            let max_priority_fee_per_gas = bigint_to_u128(trace_max_priority_fee_per_gas)?;

            Transaction::Eip1559(TxEip1559 {
                chain_id,
                nonce,
                gas_limit,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                to,
                value,
                access_list,
                input,
            })
        }
        TxType::Eip4844 => Transaction::Eip4844(TxEip4844 {
            chain_id,
            nonce,
            gas_limit,
            max_fee_per_gas: bigint_to_u128(trace.max_fee_per_gas.clone().unwrap_or_default())?,
            max_priority_fee_per_gas: bigint_to_u128(
                trace.max_priority_fee_per_gas.clone().unwrap_or_default(),
            )?,
            placeholder: None,
            to: Address::from_slice(trace.to.as_slice()),
            value,
            access_list: compute_access_list(&trace.access_list)?,
            blob_versioned_hashes: trace
                .blob_hashes
                .iter()
                .map(|v| FixedBytes::from_slice(v.as_slice()))
                .collect(),
            max_fee_per_blob_gas: bigint_to_u128(
                trace.blob_gas_fee_cap.clone().unwrap_or_default(),
            )?,
            input,
        }),
    };

    Ok(transaction)
}

pub fn get_tx_kind(trace: &TransactionTrace) -> Result<TxKind, TransactionError> {
    let first_call = trace.calls.first().ok_or(TransactionError::MissingCall)?;

    let call_type = first_call.call_type();

    if call_type == CallType::Create {
        Ok(TxKind::Create)
    } else {
        let address = Address::from_slice(trace.to.as_slice());
        Ok(TxKind::Call(address))
    }
}

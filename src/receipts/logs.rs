use crate::receipts::error::ReceiptError;
use reth_primitives::{hex, Address, Bytes, Log, H256};
use std::convert::TryInto;

// type BlockLog = crate::protos::block::Log;
use crate::sf::ethereum::r#type::v2::Log as BlockLog;

pub(crate) fn map_logs(logs: &[BlockLog]) -> Result<Vec<Log>, ReceiptError> {
    logs.iter().map(Log::try_from).collect()
}

impl TryFrom<&BlockLog> for Log {
    type Error = ReceiptError;

    fn try_from(log: &BlockLog) -> Result<Self, Self::Error> {
        let slice: [u8; 20] = log
            .address
            .as_slice()
            .try_into()
            .map_err(|_| ReceiptError::InvalidAddress(hex::encode(log.address.clone())))?;

        let address = Address::from(slice);
        let topics = map_topics(&log.topics)?;
        let data = Bytes::from(log.data.as_slice());

        Ok(Self {
            address,
            topics,
            data,
        })
    }
}

fn map_topics(topics: &[Vec<u8>]) -> Result<Vec<H256>, ReceiptError> {
    topics.iter().map(map_topic).collect()
}

fn map_topic(topic: &Vec<u8>) -> Result<H256, ReceiptError> {
    let slice: [u8; 32] = topic
        .as_slice()
        .try_into()
        .map_err(|_| ReceiptError::InvalidTopic(hex::encode(topic)))?;
    Ok(H256::from(slice))
}

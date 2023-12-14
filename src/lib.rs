//! # Flat File decoder for Firehose
//! Crate that provides utility functions to read and verify flat files from disk.
//! The verifier currently matches computed receipts & transaction roots against the roots
//! provided in the block header. Optionally, the verifier can also check the block headers
//! against a directory of block headers in json format.

mod dbin;
pub mod error;
mod headers;
pub mod protos;
mod receipts;
mod transactions;

use crate::error::DecodeError;
use crate::headers::check_valid_header;
use crate::transactions::check_transaction_root;
use dbin::DbinFile;
use ethportal_api::types::execution::accumulator::HeaderRecord;
use protobuf::{Message, MessageField};
use protos::block::{Block, BlockHeader};
use rayon::prelude::*;
use receipts::check_receipt_root;
use ethereum_types::{H256, U256};
use serde::{Serialize, Deserialize};
use simple_log::log;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Read, Write, BufRead, BufReader};
use std::path::PathBuf;

pub enum DecodeInput {
    Path(String),
    Reader(Box<dyn Read>),
}
/**
* Decode & verify flat files from a directory or a single file.
* Input can be a directory or a file.
* headers_dir is optional but must be a directory if provided.
* If headers_dir is provided, the block headers will be verified against the files in the directory.
* Header files must be named after the block number they represent and be in json format (e.g. 123.json).
* If input is a directory, all files with the extension .dbin will be processed.
* If output is provided, the decoded blocks will be written to the directory.
* If output is not provided, the decoded blocks will not be written to disk.
**/
pub fn decode_flat_files(
    input: DecodeInput,
    output: Option<&str>,
    headers_dir: Option<&str>,
) -> Result<Vec<Block>, DecodeError> {
    match input {
        DecodeInput::Path(input) => {
            let metadata = fs::metadata(&input).map_err(DecodeError::IoError)?;

            if let Some(output) = output {
                fs::create_dir_all(output).map_err(DecodeError::IoError)?;
            }

            if metadata.is_dir() {
                decode_flat_files_dir(&input, output, headers_dir)
            } else if metadata.is_file() {
                handle_file(&PathBuf::from(input), output, headers_dir)
            } else {
                Err(DecodeError::InvalidInput)
            }
        }
        DecodeInput::Reader(reader) => {
            let blocks = extract_blocks(reader)?;
            Ok(blocks)
        }
    }
}

fn decode_flat_files_dir(
    input: &str,
    output: Option<&str>,
    headers_dir: Option<&str>,
) -> Result<Vec<Block>, DecodeError> {
    let paths = fs::read_dir(input).map_err(DecodeError::IoError)?;

    let mut blocks: Vec<Block> = vec![];
    for path in paths {
        let path = path.map_err(DecodeError::IoError)?;
        match path.path().extension() {
            Some(ext) => {
                if ext != "dbin" {
                    continue;
                }
            }
            None => continue,
        };

        println!("Processing file: {}", path.path().display());
        match handle_file(&path.path(), output, headers_dir) {
            Ok(file_blocks) => {
                blocks.extend(file_blocks);
            }
            Err(err) => {
                println!("Failed to process file: {}", err);
            }
        }
    }

    Ok(blocks)
}

/**
* Decode & verify a single flat file.
* If output is provided, the decoded blocks will be written to the directory.
* If output is not provided, the decoded blocks will not be written to disk.
* headers_dir is optional but must be a directory if provided.
* If headers_dir is provided, the block headers will be verified against the files in the directory.
* Header files must be named after the block number they represent and be in json format. (e.g. 123.json)
**/
pub fn handle_file(
    path: &PathBuf,
    output: Option<&str>,
    headers_dir: Option<&str>,
) -> Result<Vec<Block>, DecodeError> {
    let mut input_file = BufReader::new(File::open(path).map_err(DecodeError::IoError)?);
    let dbin_file = DbinFile::try_from_read(&mut input_file)?;
    if dbin_file.content_type != "ETH" {
        return Err(DecodeError::InvalidContentType(dbin_file.content_type));
    }

    let mut blocks: Vec<Block> = vec![];

    for message in dbin_file.messages {
        blocks.push(handle_block(&message, output, headers_dir)?);
    }

    Ok(blocks)
}

/**
* Decode & verify a single flat file from a buffer with its contents.
* This is useful for decoding a file that is already in memory.
* Returns a vector of all the blocks in the flat file
* (it can be a single block or 100 blocks depending on format).
**/
pub fn handle_buf(buf: &[u8]) -> Result<Vec<Block>, DecodeError> {
    let dbin_file = DbinFile::try_from_read(&mut Cursor::new(buf))?;

    let mut blocks: Vec<Block> = vec![];

    for message in dbin_file.messages {
        blocks.push(handle_block(&message, None, None)?);
    }
    Ok(blocks)
}

fn handle_block(
    message: &Vec<u8>,
    output: Option<&str>,
    headers_dir: Option<&str>,
) -> Result<Block, DecodeError> {
    let message: protos::bstream::Block = Message::parse_from_bytes(message)
        .map_err(|err| DecodeError::ProtobufError(err.to_string()))?;

    let block: Block = Message::parse_from_bytes(&message.payload_buffer)
        .map_err(|err| DecodeError::ProtobufError(err.to_string()))?;

    if let Some(headers_dir) = headers_dir {
        check_valid_header(&block, headers_dir)?;
    }
    if block.number != 0 {
        check_receipt_root(&block)?;
        check_transaction_root(&block)?;
    }

    if let Some(output) = output {
        let file_name = format!("{}/block-{}.json", output, block.number);
        let mut out_file = File::create(file_name)?;

        let block_json = protobuf_json_mapping::print_to_string(&block)
            .map_err(|err| DecodeError::ProtobufError(err.to_string()))?;

        out_file
            .write_all(block_json.as_bytes())
            .map_err(DecodeError::IoError)?;
    }

    Ok(block)
}

pub fn extract_block_headers<R: Read+BufRead>(
    mut reader: R,
) -> Result<Vec<MessageField<BlockHeader>>, DecodeError> {
    log::debug!("Reading messages");
    let dbin_file = DbinFile::try_from_read(&mut reader)?;
    log::debug!("Validating blocks");

    // Parallel processing of block headers
    dbin_file
        .messages
        .par_iter()
        .map(|message| handle_block_header(message))
        .collect()
}

pub fn extract_blocks<R: Read>(mut reader: R) -> Result<Vec<Block>, DecodeError> {
    log::debug!("Reading messages");
    let dbin_file = DbinFile::try_from_read(&mut reader)?;
    log::debug!("Validating blocks");

    // Parallel processing of block headers
    dbin_file
        .messages
        .par_iter()
        .map(|message| handle_block(message, None, None))
        .collect()
}

fn handle_block_header(message: &Vec<u8>) -> Result<MessageField<BlockHeader>, DecodeError> {
    let message: protos::bstream::Block = Message::parse_from_bytes(&message)
        .map_err(|err| DecodeError::ProtobufError(err.to_string()))?;

    let block: Block = Message::parse_from_bytes(&message.payload_buffer)
        .map_err(|err| DecodeError::ProtobufError(err.to_string()))?;

    if block.number != 0 {
        check_receipt_root(&block)?;
        check_transaction_root(&block)?;
    }

    Ok(block.header)
}

// pub fn stream_blocks<R: Read, W: Write>()
// A function which decodes blocks from a reader and writes them, serialized, to a writer
pub fn stream_blocks<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
) -> Result<(), DecodeError> {
    let dbin_file = DbinFile::try_from_read(&mut reader)?;
    for message in dbin_file.messages {
        let message: protos::bstream::Block = Message::parse_from_bytes(&message)
            .map_err(|err| DecodeError::ProtobufError(err.to_string()))?;

        let block: Block = Message::parse_from_bytes(&message.payload_buffer)
            .map_err(|err| DecodeError::ProtobufError(err.to_string()))?;

        if block.number != 0 {
            check_receipt_root(&block)?;
            check_transaction_root(&block)?;
        }

        let header_record_with_number = HeaderRecordWithNumber {
            block_hash: block.hash,
            total_difficulty: block.header.total_difficulty.as_ref().ok_or(DecodeError::InvalidInput)?.bytes.clone(),
            block_number: block.number,
        };

        let header_record_json = serde_json::to_string(&header_record_with_number)
            .map_err(|err| DecodeError::ProtobufError(err.to_string()))?;

        writer
            .write_all((header_record_json+"\n").as_bytes())
            .map_err(DecodeError::IoError)?;
        writer.flush().map_err(DecodeError::IoError)?;

    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct HeaderRecordWithNumber{
    pub block_hash: Vec<u8>,
    pub total_difficulty: Vec<u8>,
    pub block_number: u64,
}
#[cfg(test)]
mod tests {
    use crate::dbin::DbinFile;
    use crate::protos::block::Block;
    use crate::receipts::check_receipt_root;
    use crate::{handle_file, protos, receipts};
    use protobuf::Message;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::PathBuf;

    #[test]
    fn test_handle_file() {
        let path = PathBuf::from("example0017686312.dbin");

        let result = handle_file(&path, None, None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_check_valid_root_fail() {
        let path = PathBuf::from("example0017686312.dbin");
        let mut file = BufReader::new(File::open(path).expect("Failed to open file"));
        let dbin_file = DbinFile::try_from_read(&mut file).expect("Failed to parse dbin file");

        let message = dbin_file.messages[0].clone();

        let message: protos::bstream::Block =
            Message::parse_from_bytes(&message).expect("Failed to parse message");
        let mut block: Block =
            Message::parse_from_bytes(&message.payload_buffer).expect("Failed to parse block");

        block.balance_changes.pop();

        let result = check_receipt_root(&block);
        matches!(
            result,
            Err(receipts::error::ReceiptError::MismatchedRoot(_, _))
        );
    }
}

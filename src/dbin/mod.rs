pub mod error;

use crate::dbin::error::DbinFileError;
use std::io::Read;

pub struct DbinFile {
    pub version: u8,
    pub content_type: String,
    pub content_version: String,
    pub messages: Vec<Vec<u8>>,
}

impl DbinFile {
    pub fn read_header<R: Read>(read: &mut R) -> Result<(u8, String, String), DbinFileError> {
        let mut buf: [u8; 4] = [0; 4];
        read.read_exact(&mut buf)
            .map_err(DbinFileError::ReadError)?;

        let (version, content_type, content_version) = Self::read_partial_header(read)?;

        Ok((version, content_type, content_version))
    }

    pub fn read_partial_header<R: Read>(
        read: &mut R,
    ) -> Result<(u8, String, String), DbinFileError> {
        let version;
        let content_type;
        let content_version;

        let mut buf: [u8; 1] = [0; 1];
        read.read_exact(&mut buf)
            .map_err(DbinFileError::ReadError)?;

        if buf[0] == 0 {
            version = 0u8;
            let mut content_type_bytes: [u8; 3] = [0; 3];
            read.read_exact(&mut content_type_bytes)
                .map_err(DbinFileError::ReadError)?;

            content_type = String::from_utf8(Vec::from(content_type_bytes))
                .map_err(DbinFileError::InvalidUTF8)?;

            let mut content_version_bytes: [u8; 2] = [0; 2];
            read.read_exact(&mut content_version_bytes)
                .map_err(DbinFileError::ReadError)?;

            content_version = String::from_utf8(Vec::from(content_version_bytes))
                .map_err(DbinFileError::InvalidUTF8)?;
        } else {
            return Err(DbinFileError::UnsupportedDBINVersion);
        }

        Ok((version, content_type, content_version))
    }

    pub fn try_from_read<R: Read>(read: &mut R) -> Result<Self, DbinFileError> {
        let (version, content_type, content_version) = Self::read_header(read)?;
        let mut messages: Vec<Vec<u8>> = vec![];

        loop {
            match Self::read_message(read) {
                Ok(message) => messages.push(message),
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::UnexpectedEof {
                        return Ok(DbinFile {
                            version,
                            content_type,
                            content_version,
                            messages,
                        });
                    } else if err.kind() == std::io::ErrorKind::Other {
                        // Check that version, content_type, and content_version match the previous header
                        let (new_version, new_content_type, new_content_version) =
                            Self::read_partial_header(read)?;
                        if version != new_version
                            || content_type != new_content_type
                            || content_version != new_content_version
                        {
                            return Err(DbinFileError::DifferingDBINVersions);
                        }
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }
}

impl DbinFile {
    pub fn read_message<R: Read>(read: &mut R) -> Result<Vec<u8>, DbinFileError> {
        let mut size: [u8; 4] = [0; 4];
        read.read_exact(&mut size)?;

        if &size == b"dbin" {
            return Err(DbinFileError::StartOfNewDBINFile);
        }

        Ok(Self::read_content(size, read)?)
    }

    fn read_content<R: Read>(size: [u8; 4], read: &mut R) -> Result<Vec<u8>, std::io::Error> {
        let size = u32::from_be_bytes(size);
        let mut content: Vec<u8> = vec![0; size as usize];
        read.read_exact(&mut content)?;
        Ok(content)
    }
}

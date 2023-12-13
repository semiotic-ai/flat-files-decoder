pub mod error;

use crate::dbin::error::DbinFileError;
use std::io::{Read, Seek, BufRead};

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

    pub fn read_partial_header<R: Read>(read: &mut R) -> Result<(u8, String, String), DbinFileError> {
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

            content_type =
                String::from_utf8(Vec::from(content_type_bytes)).map_err(DbinFileError::InvalidUTF8)?;

            let mut content_version_bytes: [u8; 2] = [0; 2];
            read.read_exact(&mut content_version_bytes)
                .map_err(DbinFileError::ReadError)?;

            content_version =
                String::from_utf8(Vec::from(content_version_bytes)).map_err(DbinFileError::InvalidUTF8)?;
        }
        else {
            return Err(DbinFileError::UnsupportedDBINVersion)
        }

        Ok((version, content_type, content_version))
    }

    pub fn try_from_read<R: Read>(read: &mut R) -> Result<Self, DbinFileError> {
        let mut dbin: [u8; 4] = [0; 4];
        read.read_exact(&mut dbin)
            .map_err(DbinFileError::ReadError)?;

        let dbin = String::from_utf8(Vec::from(dbin)).map_err(DbinFileError::InvalidUTF8)?;

        if dbin != "dbin" {
            return Err(DbinFileError::InvalidDBINBytes);
        }

        let mut version: [u8; 1] = [0];
        read.read_exact(&mut version)
            .map_err(DbinFileError::ReadError)?;

        let mut content_type: [u8; 3] = [0; 3];
        read.read_exact(&mut content_type)
            .map_err(DbinFileError::ReadError)?;

        let content_type =
            String::from_utf8(Vec::from(content_type)).map_err(DbinFileError::InvalidUTF8)?;

        let mut content_version: [u8; 2] = [0; 2];
        read.read_exact(&mut content_version)
            .map_err(DbinFileError::ReadError)?;

        let content_version =
            String::from_utf8(Vec::from(content_version)).map_err(DbinFileError::InvalidUTF8)?;

        let mut messages: Vec<Vec<u8>> = vec![];

        loop {
            match Self::read_message(read) {
                Ok(message) => messages.push(message),
                Err(err) => {
                    return if err.kind() == std::io::ErrorKind::UnexpectedEof {
                        Ok(DbinFile {
                            version: version[0],
                            content_type,
                            content_version,
                            messages,
                        })
                    } else {
                        Err(DbinFileError::ReadError(err))
                    }
                }
            }
        }
    }

    pub fn try_from_read_multiple<R: Read+Seek>(read: &mut R) -> Result<Vec<Self>, DbinFileError> {
        let mut dbin_files = Vec::new();
        loop {
            let mut dbin: [u8; 4] = [0; 4];
            match read.read_exact(&mut dbin) {
                Ok(_) => {
                    let dbin = String::from_utf8(Vec::from(dbin)).map_err(DbinFileError::InvalidUTF8)?;
                    
                    if dbin != "dbin" {
                        return Err(DbinFileError::InvalidDBINBytes);
                    }
                },
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::UnexpectedEof {
                        // End of all concatenated files
                        break;
                    } else {
                        return Err(DbinFileError::ReadError(err));
                    };
                }
            }
            let mut version: [u8; 1] = [0];
            read.read_exact(&mut version)
                .map_err(DbinFileError::ReadError)?;
    
            let mut content_type: [u8; 3] = [0; 3];
            read.read_exact(&mut content_type)
                .map_err(DbinFileError::ReadError)?;
    
            let content_type =
                String::from_utf8(Vec::from(content_type)).map_err(DbinFileError::InvalidUTF8)?;
    
            let mut content_version: [u8; 2] = [0; 2];
            read.read_exact(&mut content_version)
                .map_err(DbinFileError::ReadError)?;
    
            let content_version =
                String::from_utf8(Vec::from(content_version)).map_err(DbinFileError::InvalidUTF8)?;

            // Read messages
            let mut messages: Vec<Vec<u8>> = vec![];
            loop {
                // Peek the next 4 bytes to check if it's a new 'dbin' header
                let mut next_header: [u8; 4] = [0; 4];
                if let Ok(_) = read.read_exact(&mut next_header) {
                    if next_header == *b"dbin" {
                        // Roll back the read operation as it's the start of a new file
                        read.seek(std::io::SeekFrom::Current(-4))?;
                        break; // Exit message reading loop
                    } else {
                        // Not a new file header, roll back and read the message
                        read.seek(std::io::SeekFrom::Current(-4))?;
                    }
                }
                match Self::read_message(read) {
                    Ok(message) => messages.push(message),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::UnexpectedEof {
                            // End of current DBIN file, proceed to next
                            break;
                        } else {
                            // An error other than EOF, propagate it
                            return Err(DbinFileError::ReadError(err));
                        }
                    }
                }
            }

            dbin_files.push(DbinFile {
                version: version[0],
                content_type,
                content_version,
                messages,
            });
        }

    Ok(dbin_files)
    }
}

impl DbinFile {
    pub fn read_message<R: Read>(read: &mut R) -> Result<Vec<u8>, std::io::Error> {
        let mut size: [u8; 4] = [0; 4];
        read.read_exact(&mut size)?;

        let size = u32::from_be_bytes(size);

        let mut content: Vec<u8> = vec![0; size as usize];
        read.read_exact(&mut content)?;

        Ok(content)
    }

    pub fn read_message_streaming<R: Read>(read: &mut R) -> Result<Option<Vec<u8>>, std::io::Error> {
        let mut size: [u8; 4] = [0; 4];
        read.read_exact(&mut size)?;
        if &size == b"dbin" {
            return Ok(None);
        } else {
            let size = u32::from_be_bytes(size);

            let mut content: Vec<u8> = vec![0; size as usize];
            read.read_exact(&mut content)?;
            return Ok(Some(content));
        }
    }
}

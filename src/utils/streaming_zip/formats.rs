use anyhow::Result;
use bytes::BytesMut;
use std::time::SystemTime;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::constants::{
    ZIP_CENTRAL_DIR_HEADER_SIGNATURE as CENTRAL_DIR_HEADER_SIGNATURE,
    ZIP_END_OF_CENTRAL_DIR_SIGNATURE as END_OF_CENTRAL_DIR_SIGNATURE,
    ZIP_LOCAL_FILE_HEADER_SIGNATURE as LOCAL_FILE_HEADER_SIGNATURE,
};

// Re-export these constants publicly
pub use crate::constants::{
    ZIP_COMPRESSION_METHOD_DEFLATE as COMPRESSION_METHOD_DEFLATE,
    ZIP_COMPRESSION_METHOD_STORE as COMPRESSION_METHOD_STORE,
    ZIP_DEFAULT_BIT_FLAG as DEFAULT_BIT_FLAG, ZIP_VERSION_MADE_BY as VERSION_MADE_BY,
    ZIP_VERSION_NEEDED as VERSION_NEEDED,
};

/// ZIP file entry information
pub struct ZipEntry {
    pub name: String,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub crc32: u32,
    pub offset: u32,
    pub compression_method: u16,
    pub last_mod_time: u16,
    pub last_mod_date: u16,
}

/// File options for ZIP entries
pub struct FileOptions {
    pub compression_method: CompressionMethod,
    pub last_modified: Option<SystemTime>,
}

/// Compression methods
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CompressionMethod {
    Stored,
    Deflated,
}

impl Default for FileOptions {
    fn default() -> Self {
        Self {
            compression_method: CompressionMethod::Deflated,
            last_modified: None,
        }
    }
}

/// Local file header structure
pub struct LocalFileHeader {
    pub version_needed: u16,
    pub bit_flag: u16,
    pub compression_method: u16,
    pub last_mod_time: u16,
    pub last_mod_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
}

impl LocalFileHeader {
    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<u32> {
        let mut bytes = BytesMut::new();

        // Signature
        bytes.extend_from_slice(&LOCAL_FILE_HEADER_SIGNATURE.to_le_bytes());

        // Version needed
        bytes.extend_from_slice(&self.version_needed.to_le_bytes());

        // Bit flag
        bytes.extend_from_slice(&self.bit_flag.to_le_bytes());

        // Compression method
        bytes.extend_from_slice(&self.compression_method.to_le_bytes());

        // Last mod time and date
        bytes.extend_from_slice(&self.last_mod_time.to_le_bytes());
        bytes.extend_from_slice(&self.last_mod_date.to_le_bytes());

        // CRC32
        bytes.extend_from_slice(&self.crc32.to_le_bytes());

        // Compressed size
        bytes.extend_from_slice(&self.compressed_size.to_le_bytes());

        // Uncompressed size
        bytes.extend_from_slice(&self.uncompressed_size.to_le_bytes());

        // File name length
        bytes.extend_from_slice(&(self.file_name.len() as u16).to_le_bytes());

        // Extra field length
        bytes.extend_from_slice(&(self.extra_field.len() as u16).to_le_bytes());

        // File name
        bytes.extend_from_slice(&self.file_name);

        // Extra field
        bytes.extend_from_slice(&self.extra_field);

        // Write to output
        writer.write_all(&bytes).await?;

        Ok(bytes.len() as u32)
    }
}

/// Central directory header structure
pub struct CentralDirectoryHeader {
    pub version_made_by: u16,
    pub version_needed: u16,
    pub bit_flag: u16,
    pub compression_method: u16,
    pub last_mod_time: u16,
    pub last_mod_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub disk_number_start: u16,
    pub internal_file_attributes: u16,
    pub external_file_attributes: u32,
    pub local_header_offset: u32,
    pub file_name: Vec<u8>,
    pub extra_field: Vec<u8>,
    pub file_comment: Vec<u8>,
}

impl CentralDirectoryHeader {
    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<u32> {
        let mut bytes = BytesMut::new();

        // Signature
        bytes.extend_from_slice(&CENTRAL_DIR_HEADER_SIGNATURE.to_le_bytes());

        // Version made by
        bytes.extend_from_slice(&self.version_made_by.to_le_bytes());

        // Version needed
        bytes.extend_from_slice(&self.version_needed.to_le_bytes());

        // Bit flag
        bytes.extend_from_slice(&self.bit_flag.to_le_bytes());

        // Compression method
        bytes.extend_from_slice(&self.compression_method.to_le_bytes());

        // Last mod time and date
        bytes.extend_from_slice(&self.last_mod_time.to_le_bytes());
        bytes.extend_from_slice(&self.last_mod_date.to_le_bytes());

        // CRC32
        bytes.extend_from_slice(&self.crc32.to_le_bytes());

        // Compressed size
        bytes.extend_from_slice(&self.compressed_size.to_le_bytes());

        // Uncompressed size
        bytes.extend_from_slice(&self.uncompressed_size.to_le_bytes());

        // File name length
        bytes.extend_from_slice(&(self.file_name.len() as u16).to_le_bytes());

        // Extra field length
        bytes.extend_from_slice(&(self.extra_field.len() as u16).to_le_bytes());

        // File comment length
        bytes.extend_from_slice(&(self.file_comment.len() as u16).to_le_bytes());

        // Disk number start
        bytes.extend_from_slice(&self.disk_number_start.to_le_bytes());

        // Internal file attributes
        bytes.extend_from_slice(&self.internal_file_attributes.to_le_bytes());

        // External file attributes
        bytes.extend_from_slice(&self.external_file_attributes.to_le_bytes());

        // Local header offset
        bytes.extend_from_slice(&self.local_header_offset.to_le_bytes());

        // File name
        bytes.extend_from_slice(&self.file_name);

        // Extra field
        bytes.extend_from_slice(&self.extra_field);

        // File comment
        bytes.extend_from_slice(&self.file_comment);

        // Write to output
        writer.write_all(&bytes).await?;

        Ok(bytes.len() as u32)
    }
}

/// End of central directory record structure
pub struct EndOfCentralDirectoryRecord {
    pub disk_number: u16,
    pub central_dir_disk: u16,
    pub disk_entries: u16,
    pub total_entries: u16,
    pub central_dir_size: u32,
    pub central_dir_offset: u32,
    pub comment: Vec<u8>,
}

impl EndOfCentralDirectoryRecord {
    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<u32> {
        let mut bytes = BytesMut::new();

        // Signature
        bytes.extend_from_slice(&END_OF_CENTRAL_DIR_SIGNATURE.to_le_bytes());

        // Disk number
        bytes.extend_from_slice(&self.disk_number.to_le_bytes());

        // Central directory disk
        bytes.extend_from_slice(&self.central_dir_disk.to_le_bytes());

        // Disk entries
        bytes.extend_from_slice(&self.disk_entries.to_le_bytes());

        // Total entries
        bytes.extend_from_slice(&self.total_entries.to_le_bytes());

        // Central directory size
        bytes.extend_from_slice(&self.central_dir_size.to_le_bytes());

        // Central directory offset
        bytes.extend_from_slice(&self.central_dir_offset.to_le_bytes());

        // Comment length
        bytes.extend_from_slice(&(self.comment.len() as u16).to_le_bytes());

        // Comment
        bytes.extend_from_slice(&self.comment);

        // Write to output
        writer.write_all(&bytes).await?;

        Ok(bytes.len() as u32)
    }
}

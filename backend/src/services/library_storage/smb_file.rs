//! A file on a share held open for positioned reads and writes, which the torrent client needs
//! (it writes blocks as peers send them and reads them back to check pieces and to seed).
//!
//! The handle keeps its own clone of the connection, which multiplexes every request over the
//! one signed-in session, so reads and writes from several threads run side by side and do not
//! hold a pooled connection while they wait. A handle whose connection dropped stays dead: every
//! call on it fails as `Unavailable` and the owner opens a fresh one. A write is never repeated.

use smb2::Tree;
use smb2::client::connection::Connection;
use smb2::msg::read::{ReadRequest, ReadResponse, SMB2_CHANNEL_NONE};
use smb2::msg::set_info::{InfoType, SetInfoRequest};
use smb2::msg::write::{WriteRequest, WriteResponse};
use smb2::pack::{ReadCursor, Unpack};
use smb2::types::status::NtStatus;
use smb2::types::{Command, CreditCharge, FileId};

use super::{StorageError, StorageErrorKind, StorageResult};

/// FileEndOfFileInformation (MS-FSCC 2.4.14).
const FILE_END_OF_FILE_INFORMATION: u8 = 20;
/// The most asked of the server in one READ or WRITE, whatever larger size it allows.
const MAX_REQUEST: u32 = 1024 * 1024;

/// Credits one request of `bytes` costs (one per 64 KiB, MS-SMB2 3.1.5.2).
fn credit_charge(bytes: u64) -> CreditCharge {
    CreditCharge(bytes.div_ceil(65536).clamp(1, u64::from(u16::MAX)) as u16)
}

pub struct SmbFile {
    conn: Connection,
    tree: Tree,
    file_id: FileId,
    max_read: u32,
    max_write: u32,
    /// The connection generation the handle was opened on; a reconnect invalidates it.
    generation: u64,
    /// For messages: the path inside the share.
    path: String,
    closed: std::sync::atomic::AtomicBool,
}

fn failed(what: &str, path: &str, error: smb2::Error) -> StorageError {
    use smb2::ErrorKind as Kind;
    let kind = match error.kind() {
        Kind::NotFound => StorageErrorKind::NotFound,
        Kind::AccessDenied | Kind::AuthRequired => StorageErrorKind::PermissionDenied,
        Kind::ConnectionLost | Kind::SessionExpired | Kind::TimedOut | Kind::Cancelled => {
            StorageErrorKind::Unavailable
        }
        _ => StorageErrorKind::Other,
    };
    StorageError::new(kind, format!("{what} '{path}' on the share: {error}"))
}

fn refused(what: &str, path: &str, status: NtStatus, command: Command) -> StorageError {
    let kind = if status == NtStatus::DISK_FULL {
        StorageErrorKind::Other
    } else if status == NtStatus::ACCESS_DENIED {
        StorageErrorKind::PermissionDenied
    } else if status == NtStatus::NETWORK_NAME_DELETED || status == NtStatus::USER_SESSION_DELETED {
        StorageErrorKind::Unavailable
    } else {
        StorageErrorKind::Other
    };
    failed(what, path, smb2::Error::Protocol { status, command }).with_kind(kind)
}

impl SmbFile {
    /// Open (creating it if `create`) the file at `path` inside the share that `tree` is
    /// connected to, for reading and writing. Returns the handle and the file's size.
    pub(super) async fn open(
        mut conn: Connection,
        tree: Tree,
        path: &str,
        create: bool,
    ) -> StorageResult<(Self, u64)> {
        let (max_read, max_write) = conn
            .params()
            .map(|params| (params.max_read_size, params.max_write_size))
            .unwrap_or((65536, 65536));
        let generation = conn.generation();
        let opened = if create {
            tree.open_file_readwrite(&mut conn, path).await
        } else {
            // Without create the file must be there: an open for read first proves it, and the
            // read-write open then cannot create it.
            match tree.open_file(&mut conn, path).await {
                Ok((file_id, _)) => {
                    let _ = tree.close_handle(&mut conn, file_id).await;
                    tree.open_file_readwrite(&mut conn, path).await
                }
                Err(error) => Err(error),
            }
        };
        let (file_id, size) = opened.map_err(|error| failed("Opening", path, error))?;
        Ok((
            Self {
                conn,
                tree,
                file_id,
                max_read: max_read.clamp(65536, MAX_REQUEST),
                max_write: max_write.clamp(65536, MAX_REQUEST),
                generation,
                path: path.to_string(),
                closed: std::sync::atomic::AtomicBool::new(false),
            },
            size,
        ))
    }

    /// Whether the handle is open on the connection that is still the live one.
    pub fn is_alive(&self) -> bool {
        !self.closed.load(std::sync::atomic::Ordering::Acquire)
            && !self.conn.is_disconnected()
            && self.conn.generation() == self.generation
    }

    fn check_alive(&self) -> StorageResult<()> {
        if self.is_alive() {
            Ok(())
        } else {
            Err(StorageError::new(
                StorageErrorKind::Unavailable,
                format!(
                    "The connection to the share dropped while '{}' was open",
                    self.path
                ),
            ))
        }
    }

    /// Read `len` bytes at `offset`. Fewer come back only at the end of the file.
    pub async fn read_at(&self, offset: u64, len: usize) -> StorageResult<Vec<u8>> {
        self.check_alive()?;
        let mut out = Vec::with_capacity(len);
        let mut pos = offset;
        let end = offset + len as u64;
        while pos < end {
            let chunk = (end - pos).min(u64::from(self.max_read)) as u32;
            let request = ReadRequest {
                padding: 0x50,
                flags: 0,
                length: chunk,
                offset: pos,
                file_id: self.file_id,
                minimum_count: 0,
                channel: SMB2_CHANNEL_NONE,
                remaining_bytes: 0,
                read_channel_info: vec![],
            };
            let frame = self
                .conn
                .execute_with_credits(
                    Command::Read,
                    &request,
                    Some(self.tree.tree_id),
                    credit_charge(u64::from(chunk)),
                )
                .await
                .map_err(|error| failed("Reading", &self.path, error))?;
            if frame.header.status == NtStatus::END_OF_FILE {
                break;
            }
            if frame.header.status != NtStatus::SUCCESS {
                return Err(refused(
                    "Reading",
                    &self.path,
                    frame.header.status,
                    Command::Read,
                ));
            }
            let response = ReadResponse::unpack(&mut ReadCursor::new(&frame.body))
                .map_err(|error| failed("Reading", &self.path, error))?;
            if response.data.is_empty() {
                break;
            }
            pos += response.data.len() as u64;
            out.extend_from_slice(&response.data);
        }
        Ok(out)
    }

    /// Write all of `data` at `offset`. A write that fails is never repeated.
    pub async fn write_at(&self, offset: u64, data: &[u8]) -> StorageResult<()> {
        self.check_alive()?;
        let mut done = 0usize;
        while done < data.len() {
            let chunk = (data.len() - done).min(self.max_write as usize);
            let request = WriteRequest {
                data_offset: 0x70,
                offset: offset + done as u64,
                file_id: self.file_id,
                channel: 0,
                remaining_bytes: 0,
                write_channel_info_offset: 0,
                write_channel_info_length: 0,
                flags: 0,
                data: data[done..done + chunk].to_vec(),
            };
            let frame = self
                .conn
                .execute_with_credits(
                    Command::Write,
                    &request,
                    Some(self.tree.tree_id),
                    credit_charge(chunk as u64),
                )
                .await
                .map_err(|error| failed("Writing", &self.path, error))?;
            if frame.header.status != NtStatus::SUCCESS {
                return Err(refused(
                    "Writing",
                    &self.path,
                    frame.header.status,
                    Command::Write,
                ));
            }
            let response = WriteResponse::unpack(&mut ReadCursor::new(&frame.body))
                .map_err(|error| failed("Writing", &self.path, error))?;
            if (response.count as usize) < chunk {
                return Err(StorageError::new(
                    StorageErrorKind::Other,
                    format!(
                        "The server wrote {} of {chunk} bytes to '{}'",
                        response.count, self.path
                    ),
                ));
            }
            done += chunk;
        }
        Ok(())
    }

    /// Make the file exactly `len` bytes long, cutting it or extending it with zeros.
    pub async fn set_len(&self, len: u64) -> StorageResult<()> {
        self.check_alive()?;
        let request = SetInfoRequest {
            info_type: InfoType::File,
            file_info_class: FILE_END_OF_FILE_INFORMATION,
            additional_information: 0,
            file_id: self.file_id,
            buffer: len.to_le_bytes().to_vec(),
        };
        let frame = self
            .conn
            .execute(Command::SetInfo, &request, Some(self.tree.tree_id))
            .await
            .map_err(|error| failed("Setting the length of", &self.path, error))?;
        if frame.header.status != NtStatus::SUCCESS {
            return Err(refused(
                "Setting the length of",
                &self.path,
                frame.header.status,
                Command::SetInfo,
            ));
        }
        Ok(())
    }

    /// Release the handle on the server, once. A dead connection has released it already. Later
    /// calls on the handle fail as `Unavailable`, so its owner opens a fresh one.
    pub async fn close(&self) {
        let was_open = self.is_alive();
        if self.closed.swap(true, std::sync::atomic::Ordering::AcqRel) {
            return;
        }
        if was_open {
            let mut conn = self.conn.clone();
            let _ = self.tree.close_handle(&mut conn, self.file_id).await;
        }
    }
}

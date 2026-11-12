//! How much room the volume under a path has (`docs/design.md` Q146).
//!
//! A folder on this machine is measured with `statvfs`, a folder on a share with SMB2 QUERY_INFO
//! `FileFsFullSizeInformation`. Both report what the caller may still write, which is less than
//! what is free when a quota or the root reserve applies: that is the number shown and alerted on.
//!
//! Several libraries usually live on one volume, so a volume has a key — the share's root
//! `smb://host/share`, or `local:<device id>` for this machine — and each key is measured once.

use std::path::{Path, PathBuf};

use super::{StorageError, StorageErrorKind, StorageResult};

/// The size of one volume as the server or kernel reports it, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeCapacity {
    pub total_bytes: u64,
    /// What Librarian may still write: after quotas and reserved blocks.
    pub available_bytes: u64,
    /// Free on the volume as a whole, which may be more than `available_bytes`.
    pub free_bytes: u64,
}

impl VolumeCapacity {
    pub fn new(total_bytes: u64, available_bytes: u64, free_bytes: u64) -> Self {
        Self {
            total_bytes,
            available_bytes: available_bytes.min(total_bytes),
            free_bytes: free_bytes.min(total_bytes),
        }
    }

    /// From an SMB2 `FileFsFullSizeInformation` reading. smb2's `free_bytes` is already the
    /// caller-available units (the quota-aware number); `total_free_bytes` is the volume's.
    pub fn from_smb(info: &smb2::FsInfo) -> Self {
        Self::new(info.total_bytes, info.free_bytes, info.total_free_bytes)
    }

    /// From a `statvfs` reading: `f_bavail` is what an unprivileged writer may use, `f_bfree`
    /// includes the root reserve.
    pub fn from_statvfs(blocks: u64, available: u64, free: u64, fragment_size: u64) -> Self {
        let size = fragment_size.max(1);
        Self::new(
            blocks.saturating_mul(size),
            available.saturating_mul(size),
            free.saturating_mul(size),
        )
    }

    /// What the meter fills: everything that can no longer be written.
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }
}

/// The nearest folder at or above `path` that exists: a download folder that has not been made
/// yet is measured where it will be.
fn nearest_existing(path: &Path) -> Option<PathBuf> {
    let mut probe = path.to_path_buf();
    loop {
        if probe.exists() {
            return Some(probe);
        }
        probe = probe.parent()?.to_path_buf();
    }
}

fn local_error(path: &Path, what: &str, error: impl std::fmt::Display) -> StorageError {
    StorageError::new(
        StorageErrorKind::Other,
        format!("Could not {what} for '{}': {error}", path.display()),
    )
}

/// `local:<device id>` for the volume holding `path` on this machine.
pub fn local_volume_key(path: &Path) -> StorageResult<String> {
    let probe = nearest_existing(path).ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::NotFound,
            format!("No part of '{}' exists", path.display()),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(&probe)
            .map_err(|error| local_error(path, "read the device", error))?;
        Ok(format!("local:{}", metadata.dev()))
    }
    #[cfg(not(unix))]
    {
        // Without device ids, the drive or root the path starts with stands for its volume.
        let root = probe
            .components()
            .next()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(format!("local:{root}"))
    }
}

/// The capacity of the volume holding `path` on this machine.
pub fn local_capacity(path: &Path) -> StorageResult<VolumeCapacity> {
    let probe = nearest_existing(path).ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::NotFound,
            format!("No part of '{}' exists", path.display()),
        )
    })?;
    #[cfg(unix)]
    {
        let c_path = std::ffi::CString::new(probe.as_os_str().as_encoded_bytes())
            .map_err(|error| local_error(path, "read the free space", error))?;
        // SAFETY: `c_path` is a valid NUL-terminated string and `stat` is a zeroed, correctly
        // sized `statvfs` that only this call writes.
        let stat = unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat) != 0 {
                return Err(local_error(
                    path,
                    "read the free space",
                    std::io::Error::last_os_error(),
                ));
            }
            stat
        };
        #[allow(clippy::unnecessary_cast)] // the field widths differ between platforms
        Ok(VolumeCapacity::from_statvfs(
            stat.f_blocks as u64,
            stat.f_bavail as u64,
            stat.f_bfree as u64,
            stat.f_frsize as u64,
        ))
    }
    #[cfg(not(unix))]
    {
        let _ = probe;
        Err(StorageError::unsupported(
            "reading free space on this platform",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statvfs_on_a_temporary_folder_reads_a_real_volume() {
        let dir = tempfile::tempdir().unwrap();
        let capacity = local_capacity(dir.path()).unwrap();
        assert!(capacity.total_bytes > 0);
        assert!(capacity.available_bytes <= capacity.free_bytes);
        assert!(capacity.free_bytes <= capacity.total_bytes);
        assert_eq!(
            capacity.used_bytes(),
            capacity.total_bytes - capacity.available_bytes
        );
    }

    #[test]
    fn a_folder_not_made_yet_is_measured_where_it_will_be() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("downloads/not/yet");
        let here = local_capacity(dir.path()).unwrap();
        let there = local_capacity(&missing).unwrap();
        assert_eq!(here.total_bytes, there.total_bytes);
        assert_eq!(
            local_volume_key(dir.path()).unwrap(),
            local_volume_key(&missing).unwrap()
        );
        assert!(local_volume_key(dir.path()).unwrap().starts_with("local:"));
    }

    #[test]
    fn statvfs_units_multiply_by_the_fragment_size() {
        let capacity = VolumeCapacity::from_statvfs(1_000, 100, 150, 4096);
        assert_eq!(capacity.total_bytes, 4_096_000);
        assert_eq!(capacity.available_bytes, 409_600);
        assert_eq!(capacity.free_bytes, 614_400);
        assert_eq!(capacity.used_bytes(), 4_096_000 - 409_600);
    }

    /// A `FileFsFullSizeInformation` answer from an 8 TB share with a dataset quota: the
    /// caller-available units (the quota) are fewer than the volume's free units.
    #[test]
    fn an_smb_reading_uses_the_callers_units_not_the_volumes() {
        let recorded: [u8; 32] = [
            0x28, 0xa5, 0x46, 0x07, 0x00, 0x00, 0x00, 0x00, // TotalAllocationUnits
            0x92, 0x65, 0x17, 0x01, 0x00, 0x00, 0x00, 0x00, // CallerAvailableAllocationUnits
            0x4a, 0xa9, 0xd1, 0x01, 0x00, 0x00, 0x00, 0x00, // ActualAvailableAllocationUnits
            0x80, 0x00, 0x00, 0x00, // SectorsPerAllocationUnit (128)
            0x00, 0x02, 0x00, 0x00, // BytesPerSector (512)
        ];
        // Decoded exactly as smb2's `Tree::fs_info` does.
        let units = |range: std::ops::Range<usize>| {
            i64::from_le_bytes(recorded[range].try_into().unwrap()) as u64
        };
        let sectors = u32::from_le_bytes(recorded[24..28].try_into().unwrap());
        let bytes_per_sector = u32::from_le_bytes(recorded[28..32].try_into().unwrap());
        let unit = sectors as u64 * bytes_per_sector as u64;
        let info = smb2::FsInfo {
            total_bytes: units(0..8) * unit,
            free_bytes: units(8..16) * unit,
            total_free_bytes: units(16..24) * unit,
            bytes_per_sector,
            sectors_per_unit: sectors,
        };
        let capacity = VolumeCapacity::from_smb(&info);
        assert_eq!(unit, 65_536);
        assert_eq!(capacity.total_bytes, 122_070_312 * 65_536); // 8 TB
        assert_eq!(capacity.available_bytes, 18_310_546 * 65_536); // 1.2 TB under the quota
        assert_eq!(capacity.free_bytes, 30_517_578 * 65_536); // 2 TB on the pool
        assert!(capacity.available_bytes < capacity.free_bytes);
    }

    #[test]
    fn a_server_that_reports_more_free_than_total_is_capped() {
        let capacity = VolumeCapacity::new(100, 250, 300);
        assert_eq!(capacity.available_bytes, 100);
        assert_eq!(capacity.used_bytes(), 0);
    }
}

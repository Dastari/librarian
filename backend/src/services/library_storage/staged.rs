//! Running a tool on a file that may be on a share.
//!
//! A tool is given a path for a local file and a loopback URL for a file on a share
//! (see [`tool_argument`]). Most tools take either. For one that does not, [`run_on_input`] runs it
//! again on a copy staged in scratch space, which is removed afterwards.

use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::Context;
use uuid::Uuid;

use super::{IntoStoragePath, is_tool_url, library_storage, tool_argument};

/// The most of a file copied to scratch space for a tool that cannot read it in place.
pub const MAX_STAGED_BYTES: u64 = 1024 * 1024 * 1024;

/// A file copied from its storage to scratch space, removed when dropped.
pub struct StagedCopy {
    path: PathBuf,
}

impl StagedCopy {
    /// Copy `source` (on any storage) to a new file in `scratch`, keeping its extension, which
    /// tools use to pick a demuxer.
    pub async fn of(source: impl IntoStoragePath, scratch: &Path) -> anyhow::Result<Self> {
        let source = source.into_storage_path();
        let extension = source
            .extension()
            .map(|extension| format!(".{extension}"))
            .unwrap_or_default();
        let staged = Self {
            path: scratch.join(format!("librarian-staged-{}{extension}", Uuid::new_v4())),
        };
        let opened = library_storage()
            .open_read(&source)
            .await
            .with_context(|| format!("Cannot read '{source}' to stage a copy"))?;
        anyhow::ensure!(
            opened.meta.size <= MAX_STAGED_BYTES,
            "'{source}' is too large to stage a copy for a tool"
        );
        let mut file = tokio::fs::File::create(&staged.path)
            .await
            .context("Cannot create the staged copy")?;
        let mut reader = opened.reader;
        tokio::io::copy(&mut reader, &mut file)
            .await
            .with_context(|| format!("Cannot stage a copy of '{source}'"))?;
        Ok(staged)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StagedCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Run `tool` with the file's tool argument (its path, or a loopback URL for a file on a share).
/// If that fails and the argument was a URL, run it once more on a staged copy of the file.
pub async fn run_on_input<T, F, Fut>(
    path: impl IntoStoragePath,
    scratch: &Path,
    tool: F,
) -> anyhow::Result<T>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let path = path.into_storage_path();
    let argument = tool_argument(&path)
        .await
        .with_context(|| format!("Cannot give a tool access to '{path}'"))?;
    match tool(argument.clone()).await {
        Ok(value) => Ok(value),
        Err(error) if is_tool_url(&argument) => {
            tracing::debug!(
                file = %path,
                error = %error,
                "A tool could not read the tool URL; trying a staged copy of '{}': {}",
                path,
                error
            );
            let staged = StagedCopy::of(&path, scratch).await?;
            tool(staged.path().to_string_lossy().into_owned()).await
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn a_staged_copy_is_the_same_bytes_with_the_same_extension_and_goes_when_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("song.flac");
        std::fs::write(&source, b"audio bytes").unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let staged = StagedCopy::of(&source, scratch.path()).await.unwrap();
        let staged_path = staged.path().to_path_buf();
        assert_eq!(staged_path.extension().unwrap(), "flac");
        assert_eq!(std::fs::read(&staged_path).unwrap(), b"audio bytes");
        drop(staged);
        assert!(!staged_path.exists());
        assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn a_local_file_is_run_once_by_path_and_never_staged() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("song.flac");
        std::fs::write(&source, b"x").unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let runs = AtomicUsize::new(0);
        let failed: anyhow::Result<()> = run_on_input(&source, scratch.path(), |argument| {
            runs.fetch_add(1, Ordering::SeqCst);
            assert_eq!(argument, source.to_string_lossy());
            async { anyhow::bail!("the tool failed") }
        })
        .await;
        assert!(failed.is_err());
        assert_eq!(
            runs.load(Ordering::SeqCst),
            1,
            "a path is not retried on a copy"
        );
        let worked = run_on_input(
            &source,
            scratch.path(),
            |argument| async move { Ok(argument) },
        )
        .await
        .unwrap();
        assert_eq!(worked, source.to_string_lossy());
    }

    #[tokio::test]
    async fn a_missing_file_cannot_be_staged() {
        let scratch = tempfile::tempdir().unwrap();
        assert!(
            StagedCopy::of("/definitely/not/here.flac", scratch.path())
                .await
                .is_err()
        );
    }
}

//! Hearing what a recording says, with `whisper-cli` from whisper.cpp. It's a separate program so the
//! backend's build doesn't grow, and it's optional: everything that listens carries on without it.
//!
//! `LIBRARIAN_WHISPER_CLI` names the program, otherwise `data/models/whisper-cli` or the one on `PATH`.
//! `LIBRARIAN_WHISPER_MODEL` names the model, otherwise `data/models/ggml-tiny.en.bin`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::process::Command;

use super::identify_rules::{HeardWord, Pause, parse_pauses, parse_whisper_words};

const LOCAL_CLI: &str = "./data/models/whisper-cli";
const LOCAL_MODEL: &str = "./data/models/ggml-tiny.en.bin";
/// Far longer than any clip takes, so only a stuck program is stopped.
const CLIP_TIMEOUT: Duration = Duration::from_secs(300);
/// Half of a four-core machine, so listening never gets in the way of playback.
const MAX_THREADS: usize = 2;

#[derive(Debug, Clone)]
pub struct Whisper {
    cli: String,
    model: PathBuf,
}

impl Whisper {
    pub fn from_env() -> Self {
        let cli = std::env::var("LIBRARIAN_WHISPER_CLI")
            .ok()
            .unwrap_or_else(|| {
                if Path::new(LOCAL_CLI).exists() {
                    LOCAL_CLI.to_string()
                } else {
                    "whisper-cli".to_string()
                }
            });
        let model = std::env::var("LIBRARIAN_WHISPER_MODEL")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(LOCAL_MODEL));
        Self { cli, model }
    }

    /// Whether it can run at all, or the plain reason it can't.
    pub async fn check(&self) -> Result<(), String> {
        if !self.model.exists() {
            return Err(format!(
                "The speech model isn't at {}",
                self.model.display()
            ));
        }
        let probe = Command::new(&self.cli)
            .arg("--help")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .status()
            .await;
        match probe {
            Ok(_) => Ok(()),
            Err(error) => Err(format!("Couldn't run {}: {error}", self.cli)),
        }
    }

    /// Each word in a 16 kHz mono recording and when it starts.
    pub async fn words(&self, wav: &Path) -> Result<Vec<HeardWord>> {
        let threads = std::thread::available_parallelism()
            .map_or(1, |count| count.get() / 2)
            .clamp(1, MAX_THREADS)
            .to_string();
        let run = Command::new("nice")
            .args(["-n", "15"])
            .arg(&self.cli)
            .arg("-m")
            .arg(&self.model)
            .arg("-f")
            .arg(wav)
            .args(["-t", &threads, "-np", "-l", "en", "-ml", "1", "-sow"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output();
        let output = tokio::time::timeout(CLIP_TIMEOUT, run)
            .await
            .context("Listening took too long")?
            .context("Couldn't run the speech program")?;
        anyhow::ensure!(output.status.success(), "The speech program failed");
        Ok(parse_whisper_words(&String::from_utf8_lossy(
            &output.stdout,
        )))
    }
}

/// The pauses in a recording, in seconds from its start.
pub async fn pauses(wav: &Path) -> Result<Vec<Pause>> {
    let output = Command::new("nice")
        .args(["-n", "15", "ffmpeg", "-hide_banner", "-nostats", "-i"])
        .arg(wav)
        .args(["-af", "silencedetect=noise=-35dB:d=0.3", "-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await
        .context("Couldn't run ffmpeg")?;
    Ok(parse_pauses(&String::from_utf8_lossy(&output.stderr)))
}

/// A stretch of a book, across as many of its files as it spans, as a recording to listen to.
pub async fn cut_clip(
    files: &[(String, f64)],
    start: f64,
    length: f64,
    output: &Path,
) -> Result<()> {
    let mut command = Command::new("nice");
    command.args(["-n", "15", "ffmpeg", "-y", "-v", "error"]);
    let mut position = 0.0;
    let mut parts = 0;
    for (path, duration) in files {
        let from = start.max(position);
        let to = (start + length).min(position + duration);
        if to > from {
            command
                .arg("-ss")
                .arg(format!("{:.3}", from - position))
                .arg("-t")
                .arg(format!("{:.3}", to - from))
                .arg("-i")
                .arg(path);
            parts += 1;
        }
        position += duration;
    }
    anyhow::ensure!(parts > 0, "Nothing to listen to at that point");
    let inputs: String = (0..parts).map(|index| format!("[{index}:a]")).collect();
    let status = command
        .arg("-filter_complex")
        .arg(format!("{inputs}concat=n={parts}:v=0:a=1[out]"))
        .args(["-map", "[out]", "-ar", "16000", "-ac", "1"])
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .status()
        .await
        .context("Couldn't run ffmpeg")?;
    anyhow::ensure!(status.success(), "ffmpeg couldn't cut that audio");
    Ok(())
}

//! Minimal CLI parsing for run mode overrides.

use std::env;

use crate::app_mode::RunMode;

#[derive(Debug, Default)]
pub struct CliOptions {
    pub run_mode_override: Option<RunMode>,
}

impl CliOptions {
    pub fn from_args() -> Self {
        let mut options = CliOptions::default();
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--tray" => options.run_mode_override = Some(RunMode::Tray),
                "--service" => options.run_mode_override = Some(RunMode::Service),
                "--server" => options.run_mode_override = Some(RunMode::Server),
                "--run-mode" => {
                    if let Some(value) = args.next() {
                        options.run_mode_override = RunMode::from_arg(&value);
                    }
                }
                _ if arg.starts_with("--run-mode=") => {
                    if let Some(value) = arg.split_once('=').map(|(_, v)| v) {
                        options.run_mode_override = RunMode::from_arg(value);
                    }
                }
                _ => {}
            }
        }
        options
    }
}

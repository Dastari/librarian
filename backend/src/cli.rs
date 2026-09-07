//! Minimal CLI parsing for server run modes and explicit schema administration.

use std::env;

use anyhow::{Context, Result};

use crate::app_mode::RunMode;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SchemaCommand {
    Plan,
    Apply {
        plan_hash: String,
        backup_snapshot_id: String,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CredentialCommand {
    Plan {
        new_key_file: String,
    },
    Rotate {
        new_key_file: String,
        backup_snapshot_id: String,
    },
}

#[derive(Debug, Default)]
pub struct CliOptions {
    pub run_mode_override: Option<RunMode>,
    pub schema_command: Option<SchemaCommand>,
    pub credential_command: Option<CredentialCommand>,
}

impl CliOptions {
    pub fn from_args() -> Result<Self> {
        Self::from_iter(env::args().skip(1))
    }

    fn from_iter(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = CliOptions::default();
        let mut args = args.into_iter();
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
                "schema" => {
                    let operation = args
                        .next()
                        .context("Missing schema operation; expected `plan` or `apply`")?;
                    options.schema_command = Some(match operation.as_str() {
                        "plan" => SchemaCommand::Plan,
                        "apply" => {
                            let mut plan_hash = None;
                            let mut backup_snapshot_id = None;
                            while let Some(flag) = args.next() {
                                match flag.as_str() {
                                    "--plan-hash" => {
                                        plan_hash = Some(
                                            args.next().context("Missing value for --plan-hash")?,
                                        )
                                    }
                                    "--backup-snapshot" => {
                                        backup_snapshot_id = Some(
                                            args.next()
                                                .context("Missing value for --backup-snapshot")?,
                                        )
                                    }
                                    _ if flag.starts_with("--plan-hash=") => {
                                        plan_hash =
                                            flag.split_once('=').map(|(_, value)| value.to_string())
                                    }
                                    _ if flag.starts_with("--backup-snapshot=") => {
                                        backup_snapshot_id =
                                            flag.split_once('=').map(|(_, value)| value.to_string())
                                    }
                                    _ => anyhow::bail!(
                                        "Unknown schema apply argument `{flag}`; expected \
                                         --plan-hash and --backup-snapshot"
                                    ),
                                }
                            }
                            SchemaCommand::Apply {
                                plan_hash: plan_hash
                                    .filter(|value| !value.trim().is_empty())
                                    .context("schema apply requires --plan-hash")?,
                                backup_snapshot_id: backup_snapshot_id
                                    .filter(|value| !value.trim().is_empty())
                                    .context("schema apply requires --backup-snapshot")?,
                            }
                        }
                        _ => anyhow::bail!(
                            "Unknown schema operation `{operation}`; expected `plan` or `apply`"
                        ),
                    });
                    break;
                }
                "credentials" => {
                    let operation = args
                        .next()
                        .context("Missing credentials operation; expected `plan` or `rotate`")?;
                    let mut new_key_file = None;
                    let mut backup_snapshot_id = None;
                    while let Some(flag) = args.next() {
                        match flag.as_str() {
                            "--new-key-file" => {
                                new_key_file =
                                    Some(args.next().context("Missing value for --new-key-file")?)
                            }
                            "--backup-snapshot" => {
                                backup_snapshot_id = Some(
                                    args.next().context("Missing value for --backup-snapshot")?,
                                )
                            }
                            _ if flag.starts_with("--new-key-file=") => {
                                new_key_file =
                                    flag.split_once('=').map(|(_, value)| value.to_string())
                            }
                            _ if flag.starts_with("--backup-snapshot=") => {
                                backup_snapshot_id =
                                    flag.split_once('=').map(|(_, value)| value.to_string())
                            }
                            _ => anyhow::bail!(
                                "Unknown credentials argument `{flag}`; expected --new-key-file \
                                 and, for rotate, --backup-snapshot"
                            ),
                        }
                    }
                    let new_key_file = new_key_file
                        .filter(|value| !value.trim().is_empty())
                        .context("credentials command requires --new-key-file")?;
                    options.credential_command = Some(match operation.as_str() {
                        "plan" => CredentialCommand::Plan { new_key_file },
                        "rotate" => CredentialCommand::Rotate {
                            new_key_file,
                            backup_snapshot_id: backup_snapshot_id
                                .filter(|value| !value.trim().is_empty())
                                .context("credentials rotate requires --backup-snapshot")?,
                        },
                        _ => anyhow::bail!(
                            "Unknown credentials operation `{operation}`; expected `plan` or `rotate`"
                        ),
                    });
                    break;
                }
                _ => {}
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<CliOptions> {
        CliOptions::from_iter(args.iter().map(|value| value.to_string()))
    }

    #[test]
    fn parses_schema_plan() -> Result<()> {
        assert_eq!(
            parse(&["schema", "plan"])?.schema_command,
            Some(SchemaCommand::Plan)
        );
        Ok(())
    }

    #[test]
    fn schema_apply_requires_plan_hash_and_backup() {
        assert!(parse(&["schema", "apply"]).is_err());
        assert!(
            parse(&["schema", "apply", "--plan-hash", "abc"]).is_err(),
            "a reviewed plan without a verified backup must not be accepted"
        );
    }

    #[test]
    fn parses_explicit_schema_apply() -> Result<()> {
        assert_eq!(
            parse(&[
                "schema",
                "apply",
                "--plan-hash=abc",
                "--backup-snapshot",
                "snapshot-id",
            ])?
            .schema_command,
            Some(SchemaCommand::Apply {
                plan_hash: "abc".to_string(),
                backup_snapshot_id: "snapshot-id".to_string(),
            })
        );
        Ok(())
    }

    #[test]
    fn credential_rotation_requires_a_verified_backup_argument() {
        assert!(parse(&["credentials", "rotate", "--new-key-file", "/secret/new.key"]).is_err());
    }

    #[test]
    fn parses_credential_plan_and_rotation() -> Result<()> {
        assert_eq!(
            parse(&["credentials", "plan", "--new-key-file=/secret/new.key"])?.credential_command,
            Some(CredentialCommand::Plan {
                new_key_file: "/secret/new.key".to_string(),
            })
        );
        assert_eq!(
            parse(&[
                "credentials",
                "rotate",
                "--new-key-file",
                "/secret/new.key",
                "--backup-snapshot",
                "snapshot-id",
            ])?
            .credential_command,
            Some(CredentialCommand::Rotate {
                new_key_file: "/secret/new.key".to_string(),
                backup_snapshot_id: "snapshot-id".to_string(),
            })
        );
        Ok(())
    }
}

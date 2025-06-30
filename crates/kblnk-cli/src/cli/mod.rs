pub mod chat;
mod util;
mod error;

use clap::ArgAction;
use std::process::ExitCode;
use clap::{Parser, Subcommand};
use error::{CliError, Result};
use tracing::Level;
use crate::logging::{init_logging, LogArgs};

pub const CLI_NAME: &str = "KBLNK";

const CHAT_LOG_FILE_NAME: &str = "chat.log";

const KBLNK_ENV_LOG_STDOUT: &str = "KBLNK_LOG_STDOUT";

#[derive(Debug, PartialEq, Subcommand)]
pub enum CliRootCommand {
    #[command(alias("c"))]
    Chat,
}

#[derive(Debug, Parser, PartialEq, Default)]
#[command(version, about, name = crate::cli::CLI_NAME)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<CliRootCommand>,

    #[arg(long, short = 'v', action = ArgAction::Count, global = true)]
    pub verbose: u8,
}

impl Cli {
    pub async fn execute(self) -> Result<ExitCode> {
        let _log_guard = init_logging(LogArgs{
            log_level: match self.verbose > 0 {
                true => Some(
                    match self.verbose {
                        1 => Level::WARN,
                        2 => Level::INFO,
                        3 => Level::DEBUG,
                        _ => Level::TRACE,
                    }.to_string()
                ),
                false => None,
            },
            log_file_path: match self.subcommand {
                Some(CliRootCommand::Chat) => Some(CHAT_LOG_FILE_NAME.to_owned()),
                _ => match crate::logging::get_max_log_level() >= Level::DEBUG {
                    true => Some("cli.log".to_owned()),
                    false => None,
                }
            },
            log_to_stdout: std::env::var_os(KBLNK_ENV_LOG_STDOUT).is_some() || self.verbose > 0,
            delete_old_log_file: false,
        });

        let res = match self.subcommand {
            Some(cmd) => match cmd {
                CliRootCommand::Chat => chat::start_chat_session().await,
            }
            None => chat::start_chat_session().await,
        }?;

        Ok(res)
    }
}
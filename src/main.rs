use std::process::ExitCode;
use clap::Parser;

mod cli;
mod external;
mod logging;
mod util;
mod platform;

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = match cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            err.print().ok();
            return Ok(ExitCode::from(err.exit_code().try_into().unwrap_or(2)))
        }
    };

    let tokio_runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let res = tokio_runtime.block_on(cli.execute());

    match res {
        Ok(exit_code) => Ok(exit_code),
        Err(err) => {
            eprintln!("{} {err:?}", "error:");
            Ok(ExitCode::FAILURE)
        }
    }
}

mod args;
mod display;

use anyhow::Result;
use args::Cli;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let result = hanz::run(cli.into_config())?;
    display::write_result(&result)
}

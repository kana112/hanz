mod args;
mod display;
mod gencomp;

use anyhow::Result;
use args::Cli;
use clap::Parser;
use std::path::Path;

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.completions() {
        gencomp::generate(Path::new("completions"))?;
        return Ok(());
    }

    let result = hanz::run(cli.into_config()?)?;
    display::write_result(&result)
}

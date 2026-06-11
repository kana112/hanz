use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use hanz::{RunConfig, run};

#[derive(Debug, Parser)]
#[command(
    name = "hanz",
    version,
    about = "指定したディレクトリ配下から不要そうなファイルを表示します"
)]
struct Cli {
    #[arg(value_name = "DIRECTORY")]
    root: PathBuf,
    #[arg(long, required_unless_present = "hash")]
    name: bool,
    #[arg(long, required_unless_present = "name")]
    hash: bool,
    #[arg(long, value_name = "DIR")]
    collect: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = run(RunConfig::new(cli.root, cli.name, cli.hash, cli.collect))?;
    write_output(&output)
}

fn write_output(output: &str) -> Result<()> {
    io::stdout().lock().write_all(output.as_bytes())?;
    Ok(())
}

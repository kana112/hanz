use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;

use hanz::RunConfig;

#[derive(Debug, Parser)]
#[command(
    name = "hanz",
    version,
    about = "指定したディレクトリ配下から不要そうなファイルを表示します"
)]
pub(crate) struct Cli {
    #[arg(value_name = "DIRECTORY")]
    root: Option<PathBuf>,
    #[arg(long)]
    name: bool,
    #[arg(long)]
    hash: bool,
    #[arg(long)]
    completions: bool,
}

impl Cli {
    pub(crate) fn completions(&self) -> bool {
        self.completions
    }

    pub(crate) fn into_config(self) -> Result<RunConfig> {
        if !self.name && !self.hash {
            bail!("--name または --hash を指定してください");
        }
        let root = self
            .root
            .context("探索対象ディレクトリを指定してください")?;
        Ok(RunConfig::new(root, self.name, self.hash))
    }
}

use std::path::PathBuf;

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
    root: PathBuf,
    #[arg(long, required_unless_present = "hash")]
    name: bool,
    #[arg(long, required_unless_present = "name")]
    hash: bool,
    #[arg(long, value_name = "DIR")]
    collect: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    completions: bool
}

impl Cli {
    pub(crate) fn into_config(self) -> RunConfig {
        RunConfig::new(self.root, self.name, self.hash, self.collect)
    }
}

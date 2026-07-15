use std::fs::{self, File};
use std::path::Path;

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::{Shell, generate as generate_completion};

use super::args::Cli;

pub(crate) fn generate(output_dir: &Path) -> Result<()> {
    let mut command = Cli::command();
    let app_name = "hanz";

    for (shell, file_name) in [
        (Shell::Bash, "bash/hanz"),
        (Shell::Elvish, "elvish/hanz"),
        (Shell::Fish, "fish/hanz"),
        (Shell::PowerShell, "powershell/hanz"),
        (Shell::Zsh, "zsh/_hanz"),
    ] {
        let destination = output_dir.join(file_name);
        let parent = destination
            .parent()
            .context("補完ファイルの親ディレクトリを解決できません")?;
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "補完ファイルのディレクトリを作成できません: {}",
                parent.display()
            )
        })?;
        let mut file = File::create(&destination)
            .with_context(|| format!("補完ファイルを作成できません: {}", destination.display()))?;
        generate_completion(shell, &mut command, app_name, &mut file);
    }

    Ok(())
}

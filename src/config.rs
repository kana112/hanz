use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::detector::DetectionOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    root: PathBuf,
    by_name: bool,
    by_hash: bool,
    output_dir: Option<PathBuf>,
}

impl RunConfig {
    pub fn new(root: PathBuf, by_name: bool, by_hash: bool, output_dir: Option<PathBuf>) -> Self {
        Self {
            root,
            by_name,
            by_hash,
            output_dir,
        }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn output_dir(&self) -> Option<&Path> {
        self.output_dir.as_deref()
    }

    pub(crate) fn detection_options(&self) -> DetectionOptions {
        DetectionOptions {
            by_name: self.by_name,
            by_hash: self.by_hash,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        validate_root(&self.root)?;
        if !self.by_name && !self.by_hash {
            bail!("少なくとも1つの検出方法を有効にしてください");
        }
        if let Some(output_dir) = &self.output_dir {
            validate_collection_location(&self.root, output_dir)?;
        }
        Ok(())
    }
}

fn validate_root(root: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(root)
        .with_context(|| format!("探索対象を確認できません: {}", root.display()))?;
    if !metadata.file_type().is_dir() {
        bail!(
            "探索対象は通常のディレクトリを指定してください: {}",
            root.display()
        );
    }
    Ok(())
}

fn validate_collection_location(root: &Path, output_dir: &Path) -> Result<()> {
    let root = fs::canonicalize(root)
        .with_context(|| format!("探索対象を解決できません: {}", root.display()))?;
    match fs::canonicalize(output_dir) {
        Ok(output_dir) => reject_root_or_parent(&root, &output_dir),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("収集先を解決できません: {}", output_dir.display()))
        }
    }
}

fn reject_root_or_parent(root: &Path, output_dir: &Path) -> Result<()> {
    if root.starts_with(output_dir) {
        bail!(
            "探索対象またはその親ディレクトリは収集先に指定できません: {}",
            output_dir.display()
        );
    }
    Ok(())
}

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::detector::DetectionOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    root: PathBuf,
    by_name: bool,
    by_hash: bool,
}

impl RunConfig {
    pub fn new(root: PathBuf, by_name: bool, by_hash: bool) -> Self {
        Self {
            root,
            by_name,
            by_hash,
        }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
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

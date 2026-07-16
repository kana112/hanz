use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use walkdir::{DirEntry, WalkDir};

const EXCLUDED_DIRECTORIES: [&str; 4] = [".git", "target", "node_modules", ".junk-links"];

#[allow(dead_code)]
pub(crate) fn scan_files(root: &Path) -> Result<Vec<PathBuf>> {
    scan_entries(root).map(|result| result.files)
}

pub(crate) struct ScanResult {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) directories: Vec<PathBuf>,
}

pub(crate) fn scan_entries(root: &Path) -> Result<ScanResult> {
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(should_visit);
    collect_entries(walker, root)
}

fn should_visit(entry: &DirEntry) -> bool {
    entry.depth() == 0 || !is_excluded_directory(entry)
}

fn is_excluded_directory(entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        && EXCLUDED_DIRECTORIES
            .iter()
            .any(|name| entry.file_name() == OsStr::new(name))
}

fn collect_entries(
    walker: impl Iterator<Item = walkdir::Result<DirEntry>>,
    root: &Path,
) -> Result<ScanResult> {
    let mut files = Vec::new();
    let mut directories = Vec::new();
    for entry in walker {
        let entry =
            entry.with_context(|| format!("ディレクトリを探索できません: {}", root.display()))?;
        if entry.file_type().is_file() {
            files.push(entry.into_path());
        } else if entry.depth() > 0 && entry.file_type().is_dir() {
            directories.push(entry.into_path());
        }
    }
    files.sort();
    directories.sort();
    Ok(ScanResult { files, directories })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "hanz-scanner-{name}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn file(&self, relative: &str, content: &str) -> PathBuf {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, content).unwrap();
            path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn scan_returns_regular_files_recursively() {
        let root = TestDirectory::new("recursive-scan");
        let first = root.file("first.txt", "first");
        let second = root.file("nested/second.txt", "second");
        assert_eq!(scan_files(&root.0).unwrap(), vec![first, second]);
    }

    #[test]
    fn scan_skips_all_excluded_directories() {
        let root = TestDirectory::new("excluded-scan");
        let kept = root.file("kept.txt", "kept");
        for directory in EXCLUDED_DIRECTORIES {
            root.file(&format!("{directory}/ignored.txt"), "ignored");
        }
        assert_eq!(scan_files(&root.0).unwrap(), vec![kept]);
    }

    #[test]
    fn scan_skips_symbolic_links() {
        let root = TestDirectory::new("symlink-scan");
        let kept = root.file("kept.txt", "kept");
        std::os::unix::fs::symlink(&kept, root.0.join("link.txt")).unwrap();
        assert_eq!(scan_files(&root.0).unwrap(), vec![kept]);
    }

    #[test]
    fn invalid_roots_are_rejected_by_run_configuration() {
        let root = TestDirectory::new("invalid-root");
        let file = root.file("file.txt", "content");
        let link = root.0.join("directory-link");
        std::os::unix::fs::symlink(&root.0, &link).unwrap();
        assert!(crate::RunConfig::new(file, true, false).validate().is_err());
        assert!(crate::RunConfig::new(link, true, false).validate().is_err());
        assert!(
            crate::RunConfig::new(root.0.join("missing"), true, false)
                .validate()
                .is_err()
        );
    }
}

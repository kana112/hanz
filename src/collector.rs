use std::collections::{BTreeSet, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::candidate::{Candidate, CollectionResult};

#[derive(Debug)]
struct LinkTarget {
    file_name: OsString,
    path: PathBuf,
}

pub(crate) fn collect_links(
    candidates: &[Candidate],
    output_dir: &Path,
) -> Result<CollectionResult> {
    let targets = collect_link_targets(candidates)?;
    prepare_collection_directory(output_dir)?;
    create_links(&targets, output_dir)?;
    Ok(CollectionResult {
        output_dir: output_dir.to_path_buf(),
        link_count: targets.len(),
    })
}

fn collect_link_targets(candidates: &[Candidate]) -> Result<Vec<LinkTarget>> {
    candidates
        .iter()
        .map(|candidate| candidate.path.as_path())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(link_target)
        .collect()
}

fn link_target(path: &Path) -> Result<LinkTarget> {
    let target = fs::canonicalize(path)
        .with_context(|| format!("リンク先を解決できません: {}", path.display()))?;
    let file_name = path
        .file_name()
        .with_context(|| format!("ファイル名を取得できません: {}", path.display()))?;
    Ok(LinkTarget {
        file_name: file_name.to_os_string(),
        path: target,
    })
}

fn prepare_collection_directory(output_dir: &Path) -> Result<()> {
    match fs::symlink_metadata(output_dir) {
        Ok(metadata) => reset_collection_directory(output_dir, &metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            create_collection_directory(output_dir)
        }
        Err(error) => {
            Err(error).with_context(|| format!("収集先を確認できません: {}", output_dir.display()))
        }
    }
}

fn reset_collection_directory(output_dir: &Path, metadata: &Metadata) -> Result<()> {
    if !metadata.file_type().is_dir() {
        bail!(
            "収集先の既存パスは通常のディレクトリではありません: {}",
            output_dir.display()
        );
    }
    clear_collection_directory(output_dir)
}

fn create_collection_directory(output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "収集先ディレクトリを作成できません: {}",
            output_dir.display()
        )
    })
}

fn clear_collection_directory(output_dir: &Path) -> Result<()> {
    let entries = fs::read_dir(output_dir)
        .with_context(|| format!("収集先を読み込めません: {}", output_dir.display()))?;
    for entry in entries {
        let path = collection_entry_path(entry, output_dir)?;
        remove_collection_entry(&path)?;
    }
    Ok(())
}

fn collection_entry_path(entry: std::io::Result<fs::DirEntry>, root: &Path) -> Result<PathBuf> {
    entry
        .with_context(|| format!("収集先の内容を確認できません: {}", root.display()))
        .map(|entry| entry.path())
}

fn remove_collection_entry(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("収集先の項目を確認できません: {}", path.display()))?;
    if metadata.file_type().is_dir() {
        remove_directory(path)
    } else {
        remove_file(path)
    }
}

fn remove_directory(path: &Path) -> Result<()> {
    fs::remove_dir_all(path)
        .with_context(|| format!("収集先のディレクトリを消去できません: {}", path.display()))
}

fn remove_file(path: &Path) -> Result<()> {
    fs::remove_file(path)
        .with_context(|| format!("収集先の項目を消去できません: {}", path.display()))
}

fn create_links(targets: &[LinkTarget], output_dir: &Path) -> Result<()> {
    let mut used_names = HashSet::new();
    for target in targets {
        create_link(target, output_dir, &mut used_names)?;
    }
    Ok(())
}

fn create_link(
    target: &LinkTarget,
    output_dir: &Path,
    used_names: &mut HashSet<OsString>,
) -> Result<()> {
    let link_name = unique_link_name(&target.file_name, used_names);
    let link_path = output_dir.join(link_name);
    std::os::unix::fs::symlink(&target.path, &link_path).with_context(|| {
        format!(
            "シンボリックリンクを作成できません: {} -> {}",
            link_path.display(),
            target.path.display()
        )
    })
}

fn unique_link_name(file_name: &OsStr, used_names: &mut HashSet<OsString>) -> OsString {
    let original = file_name.to_os_string();
    if used_names.insert(original.clone()) {
        return original;
    }
    numbered_link_name(file_name, used_names)
}

fn numbered_link_name(file_name: &OsStr, used_names: &mut HashSet<OsString>) -> OsString {
    for number in 1_u64.. {
        let name = prefixed_file_name(number, file_name);
        if used_names.insert(name.clone()) {
            return name;
        }
    }
    unreachable!("the numeric link-name prefix is unbounded")
}

fn prefixed_file_name(number: u64, file_name: &OsStr) -> OsString {
    let mut numbered = OsString::from(format!("{number:03}_"));
    numbered.push(file_name);
    numbered
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::{CandidateKind, RunConfig};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "hanz-collector-{name}-{}-{nonce}",
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

    fn candidate(path: PathBuf) -> Candidate {
        Candidate {
            path,
            kind: CandidateKind::Name,
            reason: String::new(),
        }
    }

    fn link_names(directory: &Path) -> Vec<OsString> {
        let mut names = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn assert_all_symlinks(directory: &Path, names: &[OsString]) {
        for name in names {
            let metadata = fs::symlink_metadata(directory.join(name)).unwrap();
            assert!(metadata.file_type().is_symlink());
        }
    }

    #[test]
    fn collection_deduplicates_and_numbers_collisions() {
        let root = TestDirectory::new("collect");
        let first = root.file("first/duplicate.txt", "same");
        let second = root.file("second/duplicate.txt", "same");
        let output = root.0.join(".junk-links");
        let candidates = vec![
            candidate(first.clone()),
            candidate(first),
            candidate(second),
        ];
        let result = collect_links(&candidates, &output).unwrap();
        let names = link_names(&output);
        assert_eq!(
            names,
            vec![
                OsString::from("001_duplicate.txt"),
                OsString::from("duplicate.txt"),
            ]
        );
        assert_eq!(result.link_count, 2);
        assert_all_symlinks(&output, &names);
    }

    #[test]
    fn collection_replaces_existing_contents() {
        let root = TestDirectory::new("replace-collection");
        let source = root.file("source.txt", "source");
        let output = root.0.join(".junk-links");
        fs::create_dir_all(output.join("old-directory")).unwrap();
        fs::write(output.join("old.txt"), "old").unwrap();
        collect_links(&[candidate(source)], &output).unwrap();
        assert_eq!(link_names(&output), vec![OsString::from("source.txt")]);
    }

    #[test]
    fn collection_rejects_non_directory_destinations() {
        let root = TestDirectory::new("invalid-collection");
        let source = root.file("source.txt", "source");
        let output = root.file("output.txt", "existing");
        assert!(collect_links(&[candidate(source)], &output).is_err());
    }

    #[test]
    fn collection_rejects_symbolic_link_destination() {
        let root = TestDirectory::new("symlink-collection");
        let source = root.file("source.txt", "source");
        let real_output = root.0.join("real-output");
        let output = root.0.join("output-link");
        fs::create_dir(&real_output).unwrap();
        std::os::unix::fs::symlink(real_output, &output).unwrap();
        assert!(collect_links(&[candidate(source)], &output).is_err());
    }

    #[test]
    fn collection_rejects_root_and_its_parent() {
        let parent = TestDirectory::new("unsafe-collection");
        let root = parent.0.join("root");
        fs::create_dir(&root).unwrap();
        assert!(
            RunConfig::new(root.clone(), true, false, Some(root.clone()))
                .validate()
                .is_err()
        );
        assert!(
            RunConfig::new(root, true, false, Some(parent.0.clone()))
                .validate()
                .is_err()
        );
    }
}

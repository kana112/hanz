use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::candidate::{Candidate, CandidateKind};

pub(super) fn detect_by_hash(files: &[PathBuf]) -> Result<Vec<Candidate>> {
    let size_groups = group_files_by_size(files)?;
    let mut candidates = Vec::new();
    for paths in size_groups.values().filter(|paths| paths.len() > 1) {
        candidates.extend(detect_equal_hashes(paths)?);
    }
    Ok(candidates)
}

fn group_files_by_size(files: &[PathBuf]) -> Result<BTreeMap<u64, Vec<&PathBuf>>> {
    let mut groups: BTreeMap<u64, Vec<&PathBuf>> = BTreeMap::new();
    for path in files {
        let size = file_size(path)?;
        groups.entry(size).or_default().push(path);
    }
    Ok(groups)
}

fn file_size(path: &Path) -> Result<u64> {
    fs::metadata(path)
        .with_context(|| format!("ファイル情報を取得できません: {}", path.display()))
        .map(|metadata| metadata.len())
}

fn detect_equal_hashes(paths: &[&PathBuf]) -> Result<Vec<Candidate>> {
    let groups = group_files_by_hash(paths)?;
    let candidates = groups
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .flat_map(|(digest, paths)| candidates_for_hash(&digest, &paths))
        .collect();
    Ok(candidates)
}

fn group_files_by_hash<'a>(paths: &[&'a PathBuf]) -> Result<BTreeMap<[u8; 32], Vec<&'a PathBuf>>> {
    let mut groups: BTreeMap<[u8; 32], Vec<&PathBuf>> = BTreeMap::new();
    for path in paths {
        let digest = sha256_file(path)?;
        groups.entry(digest).or_default().push(*path);
    }
    Ok(groups)
}

fn candidates_for_hash(digest: &[u8], paths: &[&PathBuf]) -> Vec<Candidate> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| hash_candidate(path, duplicate_path(paths, index), digest))
        .collect()
}

fn duplicate_path<'a>(paths: &[&'a PathBuf], index: usize) -> &'a PathBuf {
    if index == 0 { paths[1] } else { paths[0] }
}

fn hash_candidate(path: &Path, duplicate: &Path, digest: &[u8]) -> Candidate {
    Candidate {
        path: path.to_path_buf(),
        kind: CandidateKind::Hash,
        reason: format!(
            "duplicate of: {}\nsha256: {}",
            duplicate.display(),
            digest_to_hex(digest)
        ),
    }
}

fn sha256_file(path: &Path) -> Result<[u8; 32]> {
    let file =
        File::open(path).with_context(|| format!("ファイルを開けません: {}", path.display()))?;
    hash_reader(BufReader::new(file))
        .with_context(|| format!("ファイルを読み込めません: {}", path.display()))
}

fn hash_reader(mut reader: impl Read) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(hasher.finalize().into());
        }
        hasher.update(&buffer[..read]);
    }
}

fn digest_to_hex(digest: &[u8]) -> String {
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("hanz-hash-{name}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn file(&self, relative: &str, content: &str) -> PathBuf {
            let path = self.0.join(relative);
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
    fn sha256_matches_known_digest() {
        let digest = hash_reader("abc".as_bytes()).unwrap();
        assert_eq!(
            digest_to_hex(&digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_detection_returns_only_identical_files() {
        let root = TestDirectory::new("hash");
        let first = root.file("first.txt", "same");
        let second = root.file("second.txt", "same");
        let different = root.file("different.txt", "diff");
        let candidates = detect_by_hash(&[first.clone(), second.clone(), different]).unwrap();
        let paths = candidates
            .into_iter()
            .map(|item| item.path)
            .collect::<BTreeSet<_>>();
        assert_eq!(paths, BTreeSet::from([first, second]));
    }

    #[test]
    fn hash_detection_handles_three_duplicates() {
        let root = TestDirectory::new("three-hashes");
        let files = ["a", "b", "c"]
            .map(|name| root.file(&format!("{name}.txt"), "same"))
            .to_vec();
        let candidates = detect_by_hash(&files).unwrap();
        assert_eq!(candidates.len(), 3);
        assert!(
            candidates
                .iter()
                .all(|item| item.reason.contains("sha256:"))
        );
    }

    #[test]
    fn hash_detection_rejects_missing_files() {
        let missing = PathBuf::from("/definitely/missing/hanz-file");
        assert!(detect_by_hash(&[missing]).is_err());
    }
}

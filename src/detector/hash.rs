use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::candidate::{Candidate, CandidateKind};

pub(super) fn detect_by_hash(files: &[PathBuf], directories: &[PathBuf]) -> Result<Vec<Candidate>> {
    let mut hash_cache = HashMap::new();
    let directory_candidates = detect_duplicate_directories(files, directories, &mut hash_cache)?;
    let duplicate_directories = directory_candidates
        .iter()
        .map(|candidate| candidate.path.clone())
        .collect::<Vec<_>>();

    let mut candidates = directory_candidates;
    candidates.extend(detect_equal_hashes(
        files,
        &duplicate_directories,
        &mut hash_cache,
    )?);
    Ok(candidates)
}

fn detect_duplicate_directories(
    files: &[PathBuf],
    all_directories: &[PathBuf],
    hash_cache: &mut HashMap<PathBuf, [u8; 32]>,
) -> Result<Vec<Candidate>> {
    let mut summary_groups: BTreeMap<DirectorySummary, Vec<&PathBuf>> = BTreeMap::new();
    for directory in all_directories {
        let summary = directory_summary(directory, files, all_directories)?;
        summary_groups.entry(summary).or_default().push(directory);
    }

    let mut candidates = Vec::new();
    for group_paths in summary_groups.values().filter(|paths| paths.len() > 1) {
        let mut hash_groups: BTreeMap<[u8; 32], Vec<&PathBuf>> = BTreeMap::new();
        for directory in group_paths {
            let digest = directory_hash(directory, files, all_directories, hash_cache)?;
            hash_groups.entry(digest).or_default().push(*directory);
        }
        for (digest, group_paths) in hash_groups.into_iter().filter(|(_, paths)| paths.len() > 1) {
            candidates.extend(candidates_for_hash(
                CandidateKind::DirectoryHash,
                &digest,
                &group_paths,
            ));
        }
    }

    Ok(remove_nested_directory_candidates(candidates))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct DirectorySummary {
    file_count: usize,
    directory_count: usize,
    total_size: u64,
}

fn directory_summary(
    directory: &Path,
    files: &[PathBuf],
    directories: &[PathBuf],
) -> Result<DirectorySummary> {
    let mut file_count = 0;
    let mut total_size = 0;
    for file in files.iter().filter(|file| file.starts_with(directory)) {
        file_count += 1;
        total_size += file_size(file)?;
    }
    let directory_count = directories
        .iter()
        .filter(|path| path.as_path() != directory && path.starts_with(directory))
        .count();
    Ok(DirectorySummary {
        file_count,
        directory_count,
        total_size,
    })
}

fn directory_hash(
    directory: &Path,
    files: &[PathBuf],
    directories: &[PathBuf],
    hash_cache: &mut HashMap<PathBuf, [u8; 32]>,
) -> Result<[u8; 32]> {
    let mut records = Vec::new();
    for child in directories
        .iter()
        .filter(|path| path.as_path() != directory && path.starts_with(directory))
    {
        let relative = child.strip_prefix(directory).with_context(|| {
            format!(
                "ディレクトリの相対パスを解決できません: {}",
                child.display()
            )
        })?;
        records.push(directory_record(b'd', relative, None, None));
    }
    for file in files.iter().filter(|path| path.starts_with(directory)) {
        let relative = file
            .strip_prefix(directory)
            .with_context(|| format!("ファイルの相対パスを解決できません: {}", file.display()))?;
        let size = file_size(file)?;
        let digest = sha256_file_cached(file, hash_cache)?;
        records.push(directory_record(b'f', relative, Some(size), Some(&digest)));
    }

    records.sort();
    let mut hasher = Sha256::new();
    for record in records {
        hasher.update((record.len() as u64).to_be_bytes());
        hasher.update(record);
    }
    Ok(hasher.finalize().into())
}

fn directory_record(
    kind: u8,
    relative: &Path,
    size: Option<u64>,
    digest: Option<&[u8; 32]>,
) -> Vec<u8> {
    let path = relative.as_os_str().as_bytes();
    let mut record = Vec::with_capacity(1 + 8 + path.len() + 8 + 32);
    record.push(kind);
    record.extend_from_slice(&(path.len() as u64).to_be_bytes());
    record.extend_from_slice(path);
    if let Some(size) = size {
        record.extend_from_slice(&size.to_be_bytes());
    }
    if let Some(digest) = digest {
        record.extend_from_slice(digest);
    }
    record
}

fn remove_nested_directory_candidates(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut paths = candidates
        .iter()
        .map(|candidate| candidate.path.clone())
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| (path.components().count(), path.clone()));

    let mut outermost = Vec::new();
    for path in paths {
        if !outermost
            .iter()
            .any(|parent: &PathBuf| path.starts_with(parent))
        {
            outermost.push(path);
        }
    }
    let outermost = outermost.into_iter().collect::<HashSet<_>>();
    candidates
        .into_iter()
        .filter(|candidate| outermost.contains(&candidate.path))
        .collect()
}

fn detect_equal_hashes(
    files: &[PathBuf],
    excluded_directories: &[PathBuf],
    hash_cache: &mut HashMap<PathBuf, [u8; 32]>,
) -> Result<Vec<Candidate>> {
    let size_groups = group_files_by_size(
        files
            .iter()
            .filter(|path| !is_inside_any_directory(path, excluded_directories)),
    )?;
    let mut candidates = Vec::new();
    for paths in size_groups.values().filter(|paths| paths.len() > 1) {
        candidates.extend(detect_equal_hashes_in_group(paths, hash_cache)?);
    }
    Ok(candidates)
}

fn is_inside_any_directory(path: &Path, directories: &[PathBuf]) -> bool {
    directories
        .iter()
        .any(|directory| path.starts_with(directory))
}

fn group_files_by_size<'a>(
    files: impl Iterator<Item = &'a PathBuf>,
) -> Result<BTreeMap<u64, Vec<&'a PathBuf>>> {
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

fn detect_equal_hashes_in_group(
    paths: &[&PathBuf],
    hash_cache: &mut HashMap<PathBuf, [u8; 32]>,
) -> Result<Vec<Candidate>> {
    let groups = group_files_by_hash(paths, hash_cache)?;
    let candidates = groups
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .flat_map(|(digest, paths)| candidates_for_hash(CandidateKind::Hash, &digest, &paths))
        .collect();
    Ok(candidates)
}

fn group_files_by_hash<'a>(
    paths: &[&'a PathBuf],
    hash_cache: &mut HashMap<PathBuf, [u8; 32]>,
) -> Result<BTreeMap<[u8; 32], Vec<&'a PathBuf>>> {
    let mut groups: BTreeMap<[u8; 32], Vec<&PathBuf>> = BTreeMap::new();
    for path in paths {
        let digest = sha256_file_cached(path, hash_cache)?;
        groups.entry(digest).or_default().push(*path);
    }
    Ok(groups)
}

fn sha256_file_cached(
    path: &Path,
    hash_cache: &mut HashMap<PathBuf, [u8; 32]>,
) -> Result<[u8; 32]> {
    if let Some(digest) = hash_cache.get(path) {
        return Ok(*digest);
    }
    let digest = sha256_file(path)?;
    hash_cache.insert(path.to_path_buf(), digest);
    Ok(digest)
}

fn candidates_for_hash(kind: CandidateKind, digest: &[u8], paths: &[&PathBuf]) -> Vec<Candidate> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| hash_candidate(kind, path, duplicate_path(paths, index), digest))
        .collect()
}

fn duplicate_path<'a>(paths: &[&'a PathBuf], index: usize) -> &'a PathBuf {
    if index == 0 { paths[1] } else { paths[0] }
}

fn hash_candidate(kind: CandidateKind, path: &Path, duplicate: &Path, digest: &[u8]) -> Candidate {
    Candidate {
        path: path.to_path_buf(),
        kind,
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
        let candidates = detect_by_hash(&[first.clone(), second.clone(), different], &[]).unwrap();
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
        let candidates = detect_by_hash(&files, &[]).unwrap();
        assert_eq!(candidates.len(), 3);
        assert!(
            candidates
                .iter()
                .all(|item| item.reason.contains("sha256:"))
        );
    }

    #[test]
    fn identical_directories_are_reported_once_without_nested_files() {
        let root = TestDirectory::new("directories");
        let first_directory = root.0.join("first");
        let second_directory = root.0.join("second");
        let first_file = root.file("first/nested/a.txt", "same");
        let second_file = root.file("second/nested/a.txt", "same");
        let files = vec![first_file, second_file];
        let directories = vec![
            first_directory.clone(),
            first_directory.join("nested"),
            second_directory.clone(),
            second_directory.join("nested"),
        ];

        let candidates = detect_by_hash(&files, &directories).unwrap();
        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| candidate.kind == CandidateKind::DirectoryHash)
                .map(|candidate| candidate.path.clone())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([first_directory, second_directory])
        );
        assert!(
            candidates
                .iter()
                .all(|candidate| { candidate.kind == CandidateKind::DirectoryHash })
        );
    }

    #[test]
    fn different_relative_file_names_do_not_make_equal_directories() {
        let root = TestDirectory::new("directory-paths");
        let first_directory = root.0.join("first");
        let second_directory = root.0.join("second");
        let first_file = root.file("first/a.txt", "same");
        let second_file = root.file("second/b.txt", "same");
        let files = vec![first_file.clone(), second_file.clone()];
        let directories = vec![first_directory.clone(), second_directory.clone()];

        let candidates = detect_by_hash(&files, &directories).unwrap();
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.kind == CandidateKind::Hash)
        );
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn hash_detection_rejects_missing_files() {
        let missing = PathBuf::from("/definitely/missing/hanz-file");
        assert!(detect_by_hash(&[missing], &[]).is_err());
    }
}

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs::{self, File, Metadata};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use walkdir::{DirEntry, WalkDir};

const EXCLUDED_DIRECTORIES: [&str; 4] = [".git", "target", "node_modules", ".junk-links"];
const NAME_REASON: &str = "reason: duplicate-like filename";

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub kind: CandidateKind,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Name,
    Hash,
}

impl CandidateKind {
    fn label(self) -> &'static str {
        match self {
            Self::Name => "NAME",
            Self::Hash => "HASH",
        }
    }
}

#[derive(Debug)]
struct LinkTarget {
    file_name: OsString,
    path: PathBuf,
}

pub fn run(config: RunConfig) -> Result<String> {
    validate_config(&config)?;
    let files = scan_files_excluding(&config.root, config.output_dir.as_deref())?;
    let candidates = detect_candidates(&files, config.by_name, config.by_hash)?;
    collect_if_requested(&config, &candidates)?;
    Ok(format_run_output(&candidates, config.output_dir.as_deref()))
}

fn validate_config(config: &RunConfig) -> Result<()> {
    validate_root(&config.root)?;
    if !config.by_name && !config.by_hash {
        bail!("--name または --hash を指定してください");
    }
    if let Some(output_dir) = &config.output_dir {
        validate_collection_location(&config.root, output_dir)?;
    }
    Ok(())
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

pub fn scan_files(root: &Path) -> Result<Vec<PathBuf>> {
    scan_files_excluding(root, None)
}

fn scan_files_excluding(root: &Path, output_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let output_dir = canonical_existing_path(output_dir);
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| should_visit(entry, output_dir.as_deref()));
    collect_regular_files(walker, root)
}

fn canonical_existing_path(path: Option<&Path>) -> Option<PathBuf> {
    path.and_then(|path| fs::canonicalize(path).ok())
}

fn should_visit(entry: &DirEntry, output_dir: Option<&Path>) -> bool {
    entry.depth() == 0
        || (!is_excluded_directory(entry) && !is_collection_directory(entry, output_dir))
}

fn is_collection_directory(entry: &DirEntry, output_dir: Option<&Path>) -> bool {
    entry.file_type().is_dir()
        && output_dir
            .is_some_and(|output| fs::canonicalize(entry.path()).is_ok_and(|path| path == output))
}

fn is_excluded_directory(entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        && EXCLUDED_DIRECTORIES
            .iter()
            .any(|name| entry.file_name() == OsStr::new(name))
}

fn collect_regular_files(
    walker: impl Iterator<Item = walkdir::Result<DirEntry>>,
    root: &Path,
) -> Result<Vec<PathBuf>> {
    let mut files = walker
        .map(|entry| checked_file_path(entry, root))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn checked_file_path(entry: walkdir::Result<DirEntry>, root: &Path) -> Result<Option<PathBuf>> {
    let entry =
        entry.with_context(|| format!("ディレクトリを探索できません: {}", root.display()))?;
    Ok(entry.file_type().is_file().then_some(entry.into_path()))
}

fn detect_candidates(files: &[PathBuf], by_name: bool, by_hash: bool) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    if by_name {
        candidates.extend(detect_by_name(files));
    }
    if by_hash {
        candidates.extend(detect_by_hash(files)?);
    }
    Ok(candidates)
}

pub fn detect_by_name(files: &[PathBuf]) -> Vec<Candidate> {
    files
        .iter()
        .filter(|path| has_duplicate_like_file_name(path))
        .map(|path| Candidate {
            path: path.clone(),
            kind: CandidateKind::Name,
            reason: NAME_REASON.to_owned(),
        })
        .collect()
}

fn has_duplicate_like_file_name(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(is_duplicate_like_name)
}

fn is_duplicate_like_name(file_name: &str) -> bool {
    if contains_copy_marker(file_name) {
        return true;
    }
    let stem = file_stem_or_name(file_name);
    contains_parenthesized_number(stem) || ends_with_space_number(stem)
}

fn contains_copy_marker(file_name: &str) -> bool {
    ["コピー", " copy", " Copy"]
        .into_iter()
        .any(|marker| file_name.contains(marker))
}

fn file_stem_or_name(file_name: &str) -> &str {
    Path::new(file_name)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(file_name)
}

fn contains_parenthesized_number(value: &str) -> bool {
    value
        .split(" (")
        .skip(1)
        .any(starts_with_number_and_closing_parenthesis)
}

fn starts_with_number_and_closing_parenthesis(value: &str) -> bool {
    let bytes = value.as_bytes();
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    digits > 0 && bytes.get(digits) == Some(&b')')
}

fn ends_with_space_number(value: &str) -> bool {
    let Some((prefix, number)) = value.rsplit_once(' ') else {
        return false;
    };
    !prefix.is_empty() && is_number_at_least_two(number)
}

fn is_number_at_least_two(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok_and(|number| number >= 2)
}

pub fn detect_by_hash(files: &[PathBuf]) -> Result<Vec<Candidate>> {
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
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

pub fn format_candidates(candidates: &[Candidate]) -> String {
    if candidates.is_empty() {
        return "No candidates found.\n".to_owned();
    }
    candidates.iter().map(format_candidate).collect()
}

fn format_candidate(candidate: &Candidate) -> String {
    let mut output = format!("{}  {}\n", candidate.kind.label(), candidate.path.display());
    for line in candidate.reason.lines() {
        writeln!(&mut output, "      {line}").expect("writing to a String cannot fail");
    }
    output.push('\n');
    output
}

fn format_run_output(candidates: &[Candidate], output_dir: Option<&Path>) -> String {
    let mut output = format_candidates(candidates);
    if let Some(output_dir) = output_dir {
        append_collection_summary(&mut output, candidates, output_dir);
    }
    output
}

fn append_collection_summary(output: &mut String, candidates: &[Candidate], output_dir: &Path) {
    let count = unique_candidate_count(candidates);
    writeln!(
        output,
        "COLLECT  {count} candidate link(s) in {}",
        output_dir.display()
    )
    .expect("writing to a String cannot fail");
}

fn collect_if_requested(config: &RunConfig, candidates: &[Candidate]) -> Result<()> {
    if let Some(output_dir) = &config.output_dir {
        collect_links(candidates, output_dir)?;
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

pub fn collect_links(candidates: &[Candidate], output_dir: &Path) -> Result<()> {
    let targets = collect_link_targets(candidates)?;
    prepare_collection_directory(output_dir)?;
    create_links(&targets, output_dir)
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

fn unique_candidate_count(candidates: &[Candidate]) -> usize {
    candidates
        .iter()
        .map(|candidate| candidate.path.as_path())
        .collect::<BTreeSet<_>>()
        .len()
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(test_directory_name(name, nonce));
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

    fn test_directory_name(name: &str, nonce: u128) -> String {
        format!("hanz-{name}-{}-{nonce}", std::process::id())
    }

    fn candidate(path: PathBuf, kind: CandidateKind, reason: &str) -> Candidate {
        Candidate {
            path,
            kind,
            reason: reason.to_owned(),
        }
    }

    fn assert_name_cases(names: &[&str], expected: bool) {
        for name in names {
            assert_eq!(is_duplicate_like_name(name), expected, "{name}");
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
    fn duplicate_like_names_match_requested_patterns() {
        let names = [
            "report のコピー.pdf",
            "reportコピー.pdf",
            "report copy.pdf",
            "report Copy.pdf",
            "report (1).pdf",
            "report (23).pdf",
            "report 2.pdf",
            "report 30.txt",
        ];
        assert_name_cases(&names, true);
    }

    #[test]
    fn ordinary_names_do_not_match() {
        let names = [
            "report.pdf",
            "report (x).pdf",
            "report 1.pdf",
            "copy-report.pdf",
            "report ().pdf",
        ];
        assert_name_cases(&names, false);
    }

    #[test]
    fn detect_by_name_returns_name_candidates() {
        let files = vec![PathBuf::from("report.pdf"), PathBuf::from("report (1).pdf")];
        let candidates = detect_by_name(&files);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, files[1]);
        assert_eq!(candidates[0].kind, CandidateKind::Name);
        assert_eq!(candidates[0].reason, NAME_REASON);
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
    fn root_must_be_a_regular_directory() {
        let root = TestDirectory::new("invalid-root");
        let file = root.file("file.txt", "content");
        let link = root.0.join("directory-link");
        std::os::unix::fs::symlink(&root.0, &link).unwrap();
        assert!(validate_root(&file).is_err());
        assert!(validate_root(&link).is_err());
        assert!(validate_root(&root.0.join("missing")).is_err());
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

    #[test]
    fn empty_candidate_output_is_explicit() {
        assert_eq!(format_candidates(&[]), "No candidates found.\n");
    }

    #[test]
    fn candidate_output_contains_kind_path_and_reason() {
        let candidates = vec![candidate(
            PathBuf::from("report (1).pdf"),
            CandidateKind::Name,
            NAME_REASON,
        )];
        let output = format_candidates(&candidates);
        assert!(output.contains("NAME  report (1).pdf"));
        assert!(output.contains("      reason: duplicate-like filename"));
    }

    #[test]
    fn collection_deduplicates_and_numbers_collisions() {
        let root = TestDirectory::new("collect");
        let first = root.file("first/duplicate.txt", "same");
        let second = root.file("second/duplicate.txt", "same");
        let output = root.0.join(".junk-links");
        let candidates = collection_candidates(first, second);
        collect_links(&candidates, &output).unwrap();
        let names = link_names(&output);
        assert_eq!(names, expected_collision_names());
        assert_all_symlinks(&output, &names);
    }

    fn collection_candidates(first: PathBuf, second: PathBuf) -> Vec<Candidate> {
        vec![
            candidate(first.clone(), CandidateKind::Name, ""),
            candidate(first, CandidateKind::Hash, ""),
            candidate(second, CandidateKind::Hash, ""),
        ]
    }

    fn expected_collision_names() -> Vec<OsString> {
        vec![
            OsString::from("001_duplicate.txt"),
            OsString::from("duplicate.txt"),
        ]
    }

    #[test]
    fn collection_replaces_existing_contents() {
        let root = TestDirectory::new("replace-collection");
        let source = root.file("source.txt", "source");
        let output = root.0.join(".junk-links");
        fs::create_dir_all(output.join("old-directory")).unwrap();
        fs::write(output.join("old.txt"), "old").unwrap();
        collect_links(&[candidate(source, CandidateKind::Name, "")], &output).unwrap();
        assert_eq!(link_names(&output), vec![OsString::from("source.txt")]);
    }

    #[test]
    fn collection_rejects_non_directory_destinations() {
        let root = TestDirectory::new("invalid-collection");
        let source = root.file("source.txt", "source");
        let output = root.file("output.txt", "existing");
        let candidates = [candidate(source, CandidateKind::Name, "")];
        assert!(collect_links(&candidates, &output).is_err());
    }

    #[test]
    fn collection_rejects_symbolic_link_destination() {
        let root = TestDirectory::new("symlink-collection");
        let source = root.file("source.txt", "source");
        let real_output = root.0.join("real-output");
        let output = root.0.join("output-link");
        fs::create_dir(&real_output).unwrap();
        std::os::unix::fs::symlink(real_output, &output).unwrap();
        assert!(collect_links(&[candidate(source, CandidateKind::Name, "")], &output).is_err());
    }

    #[test]
    fn collection_rejects_root_and_its_parent() {
        let parent = TestDirectory::new("unsafe-collection");
        let root = parent.0.join("root");
        fs::create_dir(&root).unwrap();
        assert!(validate_collection_location(&root, &root).is_err());
        assert!(validate_collection_location(&root, &parent.0).is_err());
    }

    #[test]
    fn run_returns_output_and_creates_links() {
        let root = TestDirectory::new("run");
        root.file("report (1).pdf", "content");
        let output = root.0.join(".junk-links");
        let config = RunConfig::new(root.0.clone(), true, false, Some(output.clone()));
        let result = run(config).unwrap();
        assert!(result.contains("NAME"));
        assert!(result.contains("COLLECT  1 candidate link(s)"));
        assert_eq!(link_names(&output), vec![OsString::from("report (1).pdf")]);
    }

    #[test]
    fn run_excludes_custom_collection_directory() {
        let root = TestDirectory::new("custom-output");
        let output = root.0.join("collected");
        root.file("collected/old (1).txt", "old");
        let config = RunConfig::new(root.0.clone(), true, false, Some(output.clone()));
        let result = run(config).unwrap();
        assert!(result.starts_with("No candidates found."));
        assert!(link_names(&output).is_empty());
    }

    #[test]
    fn run_requires_a_detection_mode() {
        let root = TestDirectory::new("mode");
        let config = RunConfig::new(root.0.clone(), false, false, None);
        assert!(run(config).is_err());
    }
}
